using Microsoft.Extensions.Logging;
using Parquet;
using Parquet.Data;
using Parquet.Schema;
using ElastiCompare.Models;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using System.Collections.Concurrent;

namespace ElastiCompare.Services;

public class ParquetService
{
    private readonly ILogger<ParquetService> _logger;

    public ParquetService(ILogger<ParquetService> logger)
    {
        _logger = logger;
    }

    /// <summary>
    /// Creates a streaming Parquet writer for large datasets
    /// </summary>
    public ParquetStreamingWriter CreateStreamingWriter(string filePath, string[] primaryKeys)
    {
        return new ParquetStreamingWriter(filePath, primaryKeys, _logger);
    }

    public SliceBasedParquetWriter CreateSliceBasedWriter(string baseFilePath, string[] primaryKeys)
    {
        return new SliceBasedParquetWriter(baseFilePath, primaryKeys, _logger);
    }

    /// <summary>
    /// Writes document data to Parquet file
    /// </summary>
    public async Task WriteDocumentDataAsync(string filePath, List<DocumentData> documents, string[] primaryKeys)
    {
        if (documents.Count == 0)
        {
            _logger.LogWarning("No documents to write to Parquet file: {FilePath}", filePath);
            return;
        }

        _logger.LogInformation("Writing {Count} documents to Parquet file: {FilePath}", documents.Count, filePath);

        // Create schema - start with hash fields
        var fields = new List<DataField>
        {
            new DataField("primary_key_hash", typeof(string)),
            new DataField("document_hash", typeof(string)),
            new DataField("has_valid_primary_keys", typeof(bool))
        };

        // Add primary key fields
        foreach (var key in primaryKeys)
        {
            fields.Add(new DataField($"pk_{key}", typeof(string)));
        }

        // Add raw document field
        fields.Add(new DataField("raw_document", typeof(string)));

        var schema = new ParquetSchema(fields);

        // Prepare data columns
        var primaryKeyHashColumn = documents.Select(d => d.PrimaryKeyHash).ToArray();
        var documentHashColumn = documents.Select(d => d.DocumentHash).ToArray();
        var hasValidPrimaryKeysColumn = documents.Select(d => d.HasValidPrimaryKeys).ToArray();

        // Prepare primary key columns
        var primaryKeyColumns = new Dictionary<string, string[]>();
        foreach (var key in primaryKeys)
        {
            primaryKeyColumns[key] = documents.Select(d =>
                d.PrimaryKeyValues.TryGetValue(key, out var value) ? value?.ToString() ?? "" : ""
            ).ToArray();
        }

        var rawDocumentColumn = documents.Select(d => d.Document.ToString(Formatting.None)).ToArray();

        // Ensure output directory exists
        var directory = Path.GetDirectoryName(filePath);
        if (!string.IsNullOrEmpty(directory) && !Directory.Exists(directory))
        {
            Directory.CreateDirectory(directory);
        }

        // Write Parquet file
        using var file = File.OpenWrite(filePath);
        using var parquetWriter = await ParquetWriter.CreateAsync(schema, file);
        parquetWriter.CompressionMethod = CompressionMethod.Snappy;

        using var groupWriter = parquetWriter.CreateRowGroup();

        // Write all columns
        await groupWriter.WriteColumnAsync(new DataColumn(fields[0], primaryKeyHashColumn));
        await groupWriter.WriteColumnAsync(new DataColumn(fields[1], documentHashColumn));
        await groupWriter.WriteColumnAsync(new DataColumn(fields[2], hasValidPrimaryKeysColumn));

        int fieldIndex = 3;
        foreach (var key in primaryKeys)
        {
            await groupWriter.WriteColumnAsync(new DataColumn(fields[fieldIndex], primaryKeyColumns[key]));
            fieldIndex++;
        }

        await groupWriter.WriteColumnAsync(new DataColumn(fields[fieldIndex], rawDocumentColumn));

        _logger.LogInformation("Successfully wrote {Count} documents to {FilePath}", documents.Count, filePath);
    }
}

/// <summary>
/// Streaming Parquet writer for large datasets to avoid memory issues
/// </summary>
public class ParquetStreamingWriter : IDisposable
{
    private readonly string _filePath;
    private readonly string[] _primaryKeys;
    private readonly ILogger _logger;
    private readonly ParquetSchema _schema;
    private readonly List<DocumentData> _buffer;
    private readonly int _bufferSize;
    private FileStream? _fileStream;
    private ParquetWriter? _parquetWriter;
    private bool _disposed = false;
    private int _totalDocuments = 0;
    private bool _headerWritten = false;

    public ParquetStreamingWriter(string filePath, string[] primaryKeys, ILogger logger, int bufferSize = 5000)
    {
        _filePath = filePath;
        _primaryKeys = primaryKeys;
        _logger = logger;
        _bufferSize = bufferSize;
        _buffer = new List<DocumentData>();

        // Create schema
        var fields = new List<DataField>
        {
            new DataField("primary_key_hash", typeof(string)),
            new DataField("document_hash", typeof(string)),
            new DataField("has_valid_primary_keys", typeof(bool))
        };

        foreach (var key in primaryKeys)
        {
            fields.Add(new DataField($"pk_{key}", typeof(string)));
        }

        fields.Add(new DataField("raw_document", typeof(string)));
        _schema = new ParquetSchema(fields);

        // Ensure output directory exists
        var directory = Path.GetDirectoryName(_filePath);
        if (!string.IsNullOrEmpty(directory) && !Directory.Exists(directory))
        {
            Directory.CreateDirectory(directory);
        }
    }

    private async Task InitializeWriterAsync()
    {
        if (_fileStream == null)
        {
            _fileStream = File.OpenWrite(_filePath);
            _parquetWriter = await ParquetWriter.CreateAsync(_schema, _fileStream);
            _parquetWriter.CompressionMethod = CompressionMethod.Snappy;
            _headerWritten = true;
        }
    }

    public async Task WriteBatchAsync(IEnumerable<DocumentData> documents)
    {
        foreach (var doc in documents)
        {
            _buffer.Add(doc);

            if (_buffer.Count >= _bufferSize)
            {
                await FlushBufferAsync();
            }
        }
    }

    private async Task FlushBufferAsync()
    {
        if (_buffer.Count == 0) return;

        await InitializeWriterAsync();

        if (_parquetWriter == null)
        {
            throw new InvalidOperationException("Parquet writer not initialized");
        }

        // Prepare data columns from buffer
        var primaryKeyHashColumn = _buffer.Select(d => d.PrimaryKeyHash).ToArray();
        var documentHashColumn = _buffer.Select(d => d.DocumentHash).ToArray();
        var hasValidPrimaryKeysColumn = _buffer.Select(d => d.HasValidPrimaryKeys).ToArray();

        var primaryKeyColumns = new Dictionary<string, string[]>();
        foreach (var key in _primaryKeys)
        {
            primaryKeyColumns[key] = _buffer.Select(d =>
                d.PrimaryKeyValues.TryGetValue(key, out var value) ? value?.ToString() ?? "" : ""
            ).ToArray();
        }

        var rawDocumentColumn = _buffer.Select(d => d.Document?.ToString(Formatting.None) ?? "").ToArray();

        // Write row group
        using var groupWriter = _parquetWriter.CreateRowGroup();

        var fields = _schema.GetDataFields();
        await groupWriter.WriteColumnAsync(new DataColumn(fields[0], primaryKeyHashColumn));
        await groupWriter.WriteColumnAsync(new DataColumn(fields[1], documentHashColumn));
        await groupWriter.WriteColumnAsync(new DataColumn(fields[2], hasValidPrimaryKeysColumn));

        int fieldIndex = 3;
        foreach (var key in _primaryKeys)
        {
            await groupWriter.WriteColumnAsync(new DataColumn(fields[fieldIndex], primaryKeyColumns[key]));
            fieldIndex++;
        }

        await groupWriter.WriteColumnAsync(new DataColumn(fields[fieldIndex], rawDocumentColumn));

        _totalDocuments += _buffer.Count;
        _logger.LogDebug("Flushed {Count} documents to {FilePath}, total: {Total}",
            _buffer.Count, _filePath, _totalDocuments);

        _buffer.Clear();
    }

    public async Task CompleteAsync()
    {
        // Flush any remaining documents
        await FlushBufferAsync();

        _logger.LogInformation("Successfully wrote {Count} documents to {FilePath}", _totalDocuments, _filePath);
    }

    public void Dispose()
    {
        if (!_disposed)
        {
            _parquetWriter?.Dispose();
            _fileStream?.Dispose();
            _disposed = true;
        }
    }
}

/// <summary>
/// Thread-safe Parquet writer that collects data from multiple threads and writes once at the end
/// Uses the proven JordanPrice pattern with ConcurrentBag
/// </summary>
public class ConcurrentParquetCollector : IDisposable
{
    private readonly string _filePath;
    private readonly string[] _primaryKeys;
    private readonly ILogger _logger;
    private readonly ConcurrentBag<string> _primaryKeyHashes = new();
    private readonly ConcurrentBag<string> _documentHashes = new();
    private readonly ConcurrentBag<bool> _hasValidPrimaryKeys = new();
    private readonly ConcurrentBag<string> _rawDocuments = new();
    private readonly List<ConcurrentBag<string>> _primaryKeyColumns;
    private volatile bool _disposed = false;

    public ConcurrentParquetCollector(string filePath, string[] primaryKeys, ILogger logger)
    {
        _filePath = filePath;
        _primaryKeys = primaryKeys;
        _logger = logger;

        // Create a ConcurrentBag for each primary key column
        _primaryKeyColumns = new List<ConcurrentBag<string>>();
        for (int i = 0; i < primaryKeys.Length; i++)
        {
            _primaryKeyColumns.Add(new ConcurrentBag<string>());
        }
    }

    public Task WriteBatchAsync(IEnumerable<DocumentData> documents)
    {
        foreach (var doc in documents)
        {
            _primaryKeyHashes.Add(doc.PrimaryKeyHash);
            _documentHashes.Add(doc.DocumentHash);
            _hasValidPrimaryKeys.Add(doc.HasValidPrimaryKeys);
            _rawDocuments.Add(doc.Document?.ToString(Formatting.None) ?? "");

            // Add primary key values to their respective columns
            for (int i = 0; i < _primaryKeys.Length; i++)
            {
                var key = _primaryKeys[i];
                var value = doc.PrimaryKeyValues.TryGetValue(key, out var pkValue) ? pkValue?.ToString() ?? "" : "";
                _primaryKeyColumns[i].Add(value);
            }
        }
        return Task.CompletedTask;
    }

    private async Task WriteParquetFileAsync(string[] primaryKeyHashes, string[] documentHashes, bool[] hasValidPrimaryKeys, string[] rawDocuments, List<string[]> primaryKeyArrays)
    {
        // Validate all arrays have the same length
        var totalCount = primaryKeyHashes.Length;
        if (documentHashes.Length != totalCount || hasValidPrimaryKeys.Length != totalCount || rawDocuments.Length != totalCount)
        {
            throw new ArgumentException("All arrays must have the same length");
        }

        foreach (var pkArray in primaryKeyArrays)
        {
            if (pkArray.Length != totalCount)
            {
                throw new ArgumentException("Primary key arrays must have the same length as other arrays");
            }
        }

        // Create schema
        var fields = new List<DataField>
        {
            new("primary_key_hash", typeof(string)),
            new("document_hash", typeof(string)),
            new("has_valid_primary_keys", typeof(bool))
        };

        foreach (var key in _primaryKeys)
        {
            fields.Add(new DataField($"pk_{key}", typeof(string)));
        }
        fields.Add(new DataField("raw_document", typeof(string)));

        var schema = new ParquetSchema(fields);

        _logger.LogInformation("Writing {Count} documents to parquet file: {FilePath}", totalCount, _filePath);

        using var stream = File.Create(_filePath);
        using var parquetWriter = await ParquetWriter.CreateAsync(schema, stream);
        parquetWriter.CompressionMethod = CompressionMethod.Snappy;

        // Write row group
        using var groupWriter = parquetWriter.CreateRowGroup();

        var fieldIndex = 0;
        await groupWriter.WriteColumnAsync(new DataColumn(fields[fieldIndex++], primaryKeyHashes));
        await groupWriter.WriteColumnAsync(new DataColumn(fields[fieldIndex++], documentHashes));
        await groupWriter.WriteColumnAsync(new DataColumn(fields[fieldIndex++], hasValidPrimaryKeys));

        for (int i = 0; i < _primaryKeys.Length; i++)
        {
            await groupWriter.WriteColumnAsync(new DataColumn(fields[fieldIndex++], primaryKeyArrays[i]));
        }

        await groupWriter.WriteColumnAsync(new DataColumn(fields[fieldIndex], rawDocuments));
    }

    public async Task CompleteAsync()
    {
        // Convert all ConcurrentBags to arrays like JordanPrice does
        var primaryKeyHashArray = _primaryKeyHashes.ToArray();
        var documentHashArray = _documentHashes.ToArray();
        var hasValidPrimaryKeysArray = _hasValidPrimaryKeys.ToArray();
        var rawDocumentArray = _rawDocuments.ToArray();

        var primaryKeyArrays = new List<string[]>();
        for (int i = 0; i < _primaryKeys.Length; i++)
        {
            primaryKeyArrays.Add(_primaryKeyColumns[i].ToArray());
        }

        var totalCount = primaryKeyHashArray.Length;
        if (totalCount > 0)
        {
            await WriteParquetFileAsync(primaryKeyHashArray, documentHashArray, hasValidPrimaryKeysArray, rawDocumentArray, primaryKeyArrays);
            _logger.LogInformation("Successfully wrote {Count} documents to {FilePath}", totalCount, _filePath);
        }
        else
        {
            _logger.LogWarning("No documents to write to {FilePath}", _filePath);
        }
    }

    public void Dispose()
    {
        if (!_disposed)
        {
            _disposed = true;
        }
    }
}

/// <summary>
/// Slice-based Parquet writer where each thread/slice writes to its own file to avoid memory accumulation
/// Files are later combined via DuckDB union queries
/// </summary>
public class SliceBasedParquetWriter : IDisposable
{
    private readonly string _filePath;
    private readonly string[] _primaryKeys;
    private readonly ILogger _logger;
    private readonly SemaphoreSlim _writeLock = new(1, 1);
    private readonly ParquetSchema _schema;
    private volatile bool _disposed = false;
    private volatile bool _fileInitialized = false;
    private int _totalDocuments = 0;

    public SliceBasedParquetWriter(string filePath, string[] primaryKeys, ILogger logger)
    {
        _filePath = filePath;
        _primaryKeys = primaryKeys;
        _logger = logger;

        // Create schema once
        var fields = new List<DataField>
        {
            new("primary_key_hash", typeof(string)),
            new("document_hash", typeof(string)),
            new("has_valid_primary_keys", typeof(bool))
        };

        foreach (var key in primaryKeys)
        {
            fields.Add(new DataField($"pk_{key}", typeof(string)));
        }
        fields.Add(new DataField("raw_document", typeof(string)));

        _schema = new ParquetSchema(fields);
    }

    public async Task WriteBatchAsync(IEnumerable<DocumentData> documents)
    {
        if (_disposed) return;

        var batch = documents.ToList();
        if (batch.Count == 0) return;

        await _writeLock.WaitAsync();
        try
        {
            await AppendBatchToParquetAsync(batch);
            _totalDocuments += batch.Count;

            // Log periodically to track progress
            if (_totalDocuments % 50000 == 0)
            {
                _logger.LogDebug("Streamed {Count} documents to {FilePath}", _totalDocuments, _filePath);
            }
        }
        finally
        {
            _writeLock.Release();
        }
    }

    private async Task AppendBatchToParquetAsync(List<DocumentData> batch)
    {
        // Prepare data arrays for this batch only
        var primaryKeyHashes = batch.Select(d => d.PrimaryKeyHash).ToArray();
        var documentHashes = batch.Select(d => d.DocumentHash).ToArray();
        var hasValidPrimaryKeys = batch.Select(d => d.HasValidPrimaryKeys).ToArray();
        var rawDocuments = batch.Select(d => d.Document?.ToString(Formatting.None) ?? "").ToArray();

        var primaryKeyArrays = new List<string[]>();
        for (int i = 0; i < _primaryKeys.Length; i++)
        {
            var key = _primaryKeys[i];
            var values = batch.Select(d =>
                d.PrimaryKeyValues.TryGetValue(key, out var value) ? value?.ToString() ?? "" : ""
            ).ToArray();
            primaryKeyArrays.Add(values);
        }

        // Determine file mode: Create for first write, Append for subsequent writes
        var fileMode = _fileInitialized ? FileMode.Append : FileMode.Create;

        using var stream = new FileStream(_filePath, fileMode, FileAccess.Write, FileShare.Read);
        using var parquetWriter = await ParquetWriter.CreateAsync(_schema, stream);
        parquetWriter.CompressionMethod = CompressionMethod.Snappy;

        // Write this batch as a row group
        using var groupWriter = parquetWriter.CreateRowGroup();

        var fieldIndex = 0;
        await groupWriter.WriteColumnAsync(new DataColumn(_schema.GetDataFields()[fieldIndex++], primaryKeyHashes));
        await groupWriter.WriteColumnAsync(new DataColumn(_schema.GetDataFields()[fieldIndex++], documentHashes));
        await groupWriter.WriteColumnAsync(new DataColumn(_schema.GetDataFields()[fieldIndex++], hasValidPrimaryKeys));

        for (int i = 0; i < _primaryKeys.Length; i++)
        {
            await groupWriter.WriteColumnAsync(new DataColumn(_schema.GetDataFields()[fieldIndex++], primaryKeyArrays[i]));
        }

        await groupWriter.WriteColumnAsync(new DataColumn(_schema.GetDataFields()[fieldIndex], rawDocuments));

        if (!_fileInitialized)
        {
            _fileInitialized = true;
            _logger.LogDebug("Initialized streaming Parquet file: {FilePath}", _filePath);
        }
    }

    public Task CompleteAsync()
    {
        _logger.LogInformation("Completed streaming {Count} documents to {FilePath}", _totalDocuments, _filePath);
        return Task.CompletedTask;
    }

    public void Dispose()
    {
        if (!_disposed)
        {
            _disposed = true;
            _writeLock?.Dispose();
        }
    }
}