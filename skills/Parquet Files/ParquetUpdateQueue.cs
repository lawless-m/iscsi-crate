using System;
using System.Threading;
using System.Threading.Tasks;
using Microsoft.Extensions.Logging;

namespace BPQuery;

public class ParquetUpdateQueue
{
    private readonly DatabaseService _databaseService;
    private readonly ILogger<ParquetUpdateQueue> _logger;
    private readonly string _parquetPath;

    private volatile bool _isProcessing = false;
    private volatile bool _hasWaitingRequest = false;
    private readonly object _queueLock = new object();

    public ParquetUpdateQueue(DatabaseService databaseService, ILogger<ParquetUpdateQueue> logger)
    {
        _databaseService = databaseService;
        _logger = logger;
        _parquetPath = @"C:\RI Services\Outputs\Parquets\rocs\event_entity.parquet";
    }

    public string RequestUpdate(string requestSource = "unknown")
    {
        lock (_queueLock)
        {
            if (!_isProcessing)
            {
                // No active request - start processing immediately
                _isProcessing = true;
                _ = ProcessUpdateAsync(requestSource);
                _logger.LogInformation("Parquet update started immediately for: {RequestSource}", requestSource);
                return "Update started immediately";
            }
            else if (!_hasWaitingRequest)
            {
                // Active request exists, but no waiting request - queue this one
                _hasWaitingRequest = true;
                _logger.LogInformation("Parquet update queued for: {RequestSource}", requestSource);
                return "Update queued (will run after current update completes)";
            }
            else
            {
                // Both active and waiting slots are filled - drop this request
                _logger.LogDebug("Parquet update request dropped for: {RequestSource} - queue full", requestSource);
                return "Update request dropped - queue full (1 active, 1 waiting)";
            }
        }
    }

    private async Task ProcessUpdateAsync(string initialRequestSource)
    {
        try
        {
            _logger.LogInformation("Starting Parquet update processing for: {RequestSource}", initialRequestSource);
            await _databaseService.UpdateSingleParquetWithNewEventsAsync(_parquetPath);
            _logger.LogInformation("Parquet update completed for: {RequestSource}", initialRequestSource);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Parquet update failed for: {RequestSource}", initialRequestSource);
        }
        finally
        {
            // Check if there's a waiting request to process
            bool shouldProcessNext = false;

            lock (_queueLock)
            {
                if (_hasWaitingRequest)
                {
                    // Clear the waiting flag but keep processing flag
                    _hasWaitingRequest = false;
                    shouldProcessNext = true;
                    _logger.LogInformation("Processing queued Parquet update request");
                }
                else
                {
                    // No waiting request - clear processing flag
                    _isProcessing = false;
                    _logger.LogDebug("Parquet update queue is now empty");
                }
            }

            // If there was a waiting request, process it immediately
            if (shouldProcessNext)
            {
                await ProcessUpdateAsync("queued-request");
            }
            else
            {
                // All done - clear processing flag if not already cleared
                lock (_queueLock)
                {
                    _isProcessing = false;
                }
            }
        }
    }

    public (bool isProcessing, bool hasWaiting) GetStatus()
    {
        lock (_queueLock)
        {
            return (_isProcessing, _hasWaitingRequest);
        }
    }
}