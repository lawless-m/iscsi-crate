using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using JordanPrice.Configuration;
using JordanPrice.Models;
using Newtonsoft.Json;
using System.Net.Http;
using System.Text;

namespace JordanPrice.Services;

public class ElasticsearchService : IElasticsearchService
{
    private readonly ILogger<ElasticsearchService> _logger;
    private readonly AppSettings _settings;
    private readonly ServiceLib.Elasticsearch _sourceClient;
    private readonly ServiceLib.Elasticsearch _destinationClient;

    private const string IndexPrefix = "price_discount_";
    private const string AliasName = "price_discount";
    private const string PricingPeriodsIndex = "pricing_periods";

    private string _newIndexName = string.Empty;

    public ElasticsearchService(ILogger<ElasticsearchService> logger, IOptions<AppSettings> appSettings)
    {
        _logger = logger;
        _settings = appSettings.Value;

        // Create filtered logger for ServiceLib to reduce verbose output
        var filteredLogger = new FilteredLogger<ElasticsearchService>(logger);

        // Create ServiceLib clients for source and destination with filtered logger
        _sourceClient = new ServiceLib.Elasticsearch(_settings.Elasticsearch.Source, filteredLogger);
        _destinationClient = new ServiceLib.Elasticsearch(_settings.Elasticsearch.Destination, filteredLogger);

        _logger.LogInformation("Elasticsearch clients configured - Source: {Source}, Destination: {Destination}",
            _settings.Elasticsearch.Source, _settings.Elasticsearch.Destination);
    }

    public async Task InitializeIndexAsync()
    {
        // Generate new timestamped index name
        _newIndexName = $"{IndexPrefix}{DateTime.Now:yyyyMMdd_HHmmss}";
        _logger.LogInformation("Creating new index: {IndexName}", _newIndexName);

        try
        {
            // Create new index without explicit mapping - let Elasticsearch auto-detect field types
            // This is safer and matches how the original index was likely created
            using var httpClient = new HttpClient();
            var createUrl = $"{_settings.Elasticsearch.Destination.TrimEnd('/')}/{_newIndexName}";

            var response = await httpClient.PutAsync(createUrl, null);

            if (response.IsSuccessStatusCode)
            {
                _logger.LogInformation("Successfully created new index {IndexName}", _newIndexName);
            }
            else
            {
                var errorContent = await response.Content.ReadAsStringAsync();
                throw new InvalidOperationException($"Failed to create index {_newIndexName}: {response.StatusCode} - {errorContent}");
            }
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error creating new index {IndexName}", _newIndexName);
            throw;
        }
    }

    public Task<PricingPeriod?> GetCurrentPricingPeriodAsync()
    {
        try
        {
            var today = DateTime.Today.ToString("yyyy-MM-dd");

            // Use the working ServiceLib.Elasticsearch approach like CRMPollerFixer
            var query = $$"""
            {
                "query": {
                    "bool": {
                        "filter": [
                            {
                                "range": {
                                    "keyInStartDate": {
                                        "lte": "{{today}}"
                                    }
                                }
                            },
                            {
                                "range": {
                                    "keyInEndDate": {
                                        "gte": "{{today}}"
                                    }
                                }
                            }
                        ]
                    }
                }
            }
            """;

            var elasticsearchClient = new ServiceLib.Elasticsearch(_settings.Elasticsearch.Source, _logger);
            var result = elasticsearchClient.Search(PricingPeriodsIndex, query);

            if (result?.hits?.hits?.Count > 0)
            {
                var source = result.hits.hits[0]._source;

                // Extract values directly from the dynamic object
                var period = new PricingPeriod
                {
                    Id = source.id?.ToString() ?? string.Empty,
                    KeyInStartDate = source.keyInStartDate?.ToString() ?? string.Empty,
                    KeyInEndDate = source.keyInEndDate?.ToString() ?? string.Empty
                };

                _logger.LogInformation("Found pricing period: Id='{Id}', Start='{Start}', End='{End}'",
                    (object)period.Id, (object)period.KeyInStartDate, (object)period.KeyInEndDate);

                return Task.FromResult<PricingPeriod?>(period);
            }

            _logger.LogWarning("No current pricing period found in Elasticsearch");
            return Task.FromResult<PricingPeriod?>(null);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error getting current pricing period from Elasticsearch");
            return Task.FromResult<PricingPeriod?>(null);
        }
    }

    public Task BulkIndexPriceDiscountsAsync(IEnumerable<PriceDiscount> priceDiscounts)
    {
        var documents = priceDiscounts.ToList();

        try
        {
            var result = _destinationClient.BulkIndex(_newIndexName, documents);
            // Don't log the result which might contain document content
            if (result == "Error")
            {
                throw new InvalidOperationException("Bulk index operation failed");
            }
            // Don't log individual batch operations - will be handled by parallel processor
            return Task.CompletedTask;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error during bulk index operation to {IndexName}", _newIndexName);
            throw;
        }
    }

    public async Task FinalizeIndexAsync()
    {
        _logger.LogInformation("Finalizing index - switching alias {AliasName} to point to {NewIndexName}", AliasName, _newIndexName);

        try
        {
            // Switch the alias to point to the new index using HTTP client
            var aliasActions = $$"""
            {
                "actions": [
                    {
                        "remove": {
                            "index": "{{IndexPrefix}}*",
                            "alias": "{{AliasName}}"
                        }
                    },
                    {
                        "add": {
                            "index": "{{_newIndexName}}",
                            "alias": "{{AliasName}}"
                        }
                    }
                ]
            }
            """;

            using var httpClient = new HttpClient();
            var aliasUrl = $"{_settings.Elasticsearch.Destination.TrimEnd('/')}/_aliases";
            var content = new StringContent(aliasActions, Encoding.UTF8, "application/json");

            var response = await httpClient.PostAsync(aliasUrl, content);

            if (response.IsSuccessStatusCode)
            {
                _logger.LogInformation("Successfully switched alias {AliasName} to {NewIndexName}", AliasName, _newIndexName);
            }
            else
            {
                var errorContent = await response.Content.ReadAsStringAsync();
                throw new InvalidOperationException($"Failed to switch alias {AliasName} to {_newIndexName}: {response.StatusCode} - {errorContent}");
            }
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error switching alias {AliasName} to {NewIndexName}", AliasName, _newIndexName);
            throw;
        }
    }

    public async Task DeleteNewIndexAsync()
    {
        if (string.IsNullOrEmpty(_newIndexName))
        {
            _logger.LogInformation("No new index to clean up");
            return;
        }

        _logger.LogInformation("Cleaning up failed index: {IndexName}", _newIndexName);

        try
        {
            using var httpClient = new HttpClient();
            var deleteUrl = $"{_settings.Elasticsearch.Destination.TrimEnd('/')}/{_newIndexName}";

            var response = await httpClient.DeleteAsync(deleteUrl);

            if (response.IsSuccessStatusCode)
            {
                _logger.LogInformation("Successfully deleted failed index {IndexName}", _newIndexName);
            }
            else
            {
                var errorContent = await response.Content.ReadAsStringAsync();
                _logger.LogWarning("Failed to delete index {IndexName}: {StatusCode} - {Error}",
                    _newIndexName, response.StatusCode, errorContent);
            }
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error deleting failed index {IndexName}", _newIndexName);
            // Don't re-throw since this is cleanup - log and continue
        }
    }

    public Task SendMailAsync(int count)
    {
        _logger.LogInformation("Successfully indexed {Count} price discount records (TODO: implement email notification)", count);
        return Task.CompletedTask;
    }

    public Task SendMailAsync(Exception ex)
    {
        _logger.LogError(ex, "Price discount indexing failed with error: {Message} (TODO: implement email notification)", ex.Message);
        return Task.CompletedTask;
    }
}