using System.Text;
using Microsoft.Extensions.Logging;
using Newtonsoft.Json;
using System.Security.Cryptography;
using Newtonsoft.Json.Linq;
using System.Linq;
using System.Collections.Generic;

namespace ServiceLib;

public class Elasticsearch(string baseUrl, ILogger logger)
{
    private static string ByteArrayToHexString(byte[] bytes)
    {
        StringBuilder sb = new StringBuilder();
        foreach (byte b in bytes)
        {
            sb.Append(b.ToString("X2"));
        }
        return sb.ToString();
    }
    private readonly HttpClient _httpClient = new HttpClient()
    {
        DefaultRequestHeaders =
        {
            {"User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36"}
        }
    };
    private readonly string _baseUrl = baseUrl;
    protected readonly ILogger _logger = logger;
    
    public dynamic DataFromEndpoint(string endpoint)
    {
        var url = $"{_baseUrl}/{endpoint}";
        _logger.LogDebug("DataFromEndpoint {url}", url);
        var response = _httpClient.GetAsync(url).Result;
        response.EnsureSuccessStatusCode();
        return response.Content.ReadAsStringAsync();
    }

    private string EscKey(string key) {
        return $"{key}:";
    }
    private string EscKeys(string key, List<(string, string)> items)
    {
        var escItems = string.Join(",", items.Select(item => $"""{item.Item1}":"{item.Item2}"""));
        return $"{key}:{{{escItems}}}";
    }

    public static string GetId(dynamic hit)
    {
        return hit._id;
    }

    public static List<string> ExtractIds(dynamic searchResults)
    {
        var ids = new List<string>();
        foreach (var hit in searchResults.hits.hits)
        {
            ids.Add((string)hit._id);
        }
        return ids;
    }

    public List<dynamic> DownloadIndex(string index)
    {
        var allData = new List<dynamic>();
        const string scrollTimeout = "1m"; // Keep the scroll context open for 1 minute
        string? scrollId = null;

        try
        {
            // Initial search request to get the first batch and a scroll_id
            var initialQuery = JsonConvert.SerializeObject(new 
            {
                query = new { match_all = new { } },
                size = 5000 // Initial batch size
            });

            var initialResponse = _httpClient.PostAsync($"{_baseUrl}/{index}/_search?scroll={scrollTimeout}", new StringContent(initialQuery, Encoding.UTF8, "application/json")).Result;
            initialResponse.EnsureSuccessStatusCode();
            var initialJson = initialResponse.Content.ReadAsStringAsync().Result;
            dynamic initialResults = JsonConvert.DeserializeObject<dynamic>(initialJson)!;
            
            scrollId = initialResults._scroll_id;
            allData.AddRange(initialResults.hits.hits);
            _logger.LogInformation("Downloaded {count} documents from {index} (initial scroll), total so far: {total}", (int)initialResults.hits.hits.Count, index, (int)allData.Count);

            // Loop to fetch subsequent scroll pages
            while (true)
            {
                if (string.IsNullOrEmpty(scrollId))
                {
                    _logger.LogWarning("Scroll ID is null or empty, breaking from scroll loop.");
                    break;
                }

                var scrollQuery = JsonConvert.SerializeObject(new 
                {
                    scroll = scrollTimeout,
                    scroll_id = scrollId
                });

                var scrollResponse = _httpClient.PostAsync($"{_baseUrl}/_search/scroll", new StringContent(scrollQuery, Encoding.UTF8, "application/json")).Result;
                scrollResponse.EnsureSuccessStatusCode();
                var scrollJson = scrollResponse.Content.ReadAsStringAsync().Result;
                dynamic scrollResults = JsonConvert.DeserializeObject<dynamic>(scrollJson)!;

                if (scrollResults.hits.hits.Count == 0)
                {
                    break; // No more documents
                }

                allData.AddRange(scrollResults.hits.hits);
                scrollId = scrollResults._scroll_id; // Update scroll ID for next iteration
                _logger.LogInformation("Downloaded {count} documents from {index} (scrolling), total so far: {total}", (int)scrollResults.hits.hits.Count, index, (int)allData.Count);
            }
        }
        catch (Exception ex)
        {
            _logger.LogError("Error downloading index {index} using Scroll API: {message}", index, ex.Message);
            // Decide if you want to re-throw or handle gracefully
            throw; 
        }
        finally
        {
            // Always clear the scroll context
            if (!string.IsNullOrEmpty(scrollId))
            {
                try
                {
                    var clearScrollRequest = new HttpRequestMessage(HttpMethod.Delete, $"{_baseUrl}/_search/scroll");
                    clearScrollRequest.Content = new StringContent(JsonConvert.SerializeObject(new { scroll_id = new[] { scrollId } }), Encoding.UTF8, "application/json");

                    var clearResponse = _httpClient.SendAsync(clearScrollRequest).Result;
                    clearResponse.EnsureSuccessStatusCode();
                    _logger.LogDebug("Cleared scroll context for scroll ID: {scrollId}", scrollId);
                }
                catch (Exception ex)
                {
                    _logger.LogError("Error clearing scroll context {scrollId}: {message}", scrollId, ex.Message);
                }
            }
        }

        return allData.Select(hit => hit._source).ToList();
    }

    public void AsJsonFile(List<dynamic> data, string path)
    {
        var json = JsonConvert.SerializeObject(data, Formatting.None);
        File.WriteAllText(path, json);
    }

    public string? GetCanonicalJsonString(dynamic obj)
    {
        if (obj == null)
        {
            return null;
        }

        JObject jObject = JObject.FromObject(obj);
        _logger.LogDebug("GetCanonicalJsonString: Initial JObject for {Id}: {Json}", (string?)(jObject["Code"]?.ToString()) ?? "Unknown", (string)jObject.ToString(Formatting.None));

        // Remove properties that are null or empty strings
        var propertiesToRemove = jObject.Properties()
                                        .Where(p => p.Value.Type == JTokenType.Null || 
                                                    (p.Value.Type == JTokenType.String && string.IsNullOrEmpty(p.Value.ToString())))
                                        .ToList();
        
        if (propertiesToRemove.Any())
        {
            _logger.LogDebug("GetCanonicalJsonString: Properties to remove for {Id}: {Props}", (string?)(jObject["Code"]?.ToString()) ?? "Unknown", string.Join(", ", propertiesToRemove.Select(p => p.Name)));
        }

        foreach (var prop in propertiesToRemove)
        {
            prop.Remove();
        }

        var orderedProperties = ((IEnumerable<JProperty>)jObject.Properties()).OrderBy(p => p.Name, StringComparer.Ordinal);
        var orderedJObject = new JObject(orderedProperties);
        
        CleanJToken(orderedJObject); // Apply cleaning for hidden characters and trimming

        _logger.LogDebug("GetCanonicalJsonString: Final JObject before serialization for {Id}: {Json}", (string?)(orderedJObject["Code"]?.ToString()) ?? "Unknown", (string)orderedJObject.ToString(Formatting.None));

        var settings = new JsonSerializerSettings
        {
            NullValueHandling = NullValueHandling.Ignore,
            Formatting = Formatting.None
        };
        return JsonConvert.SerializeObject(orderedJObject, settings);
    }

    public JToken CleanJToken(JToken token)
    {
        if (token.Type == JTokenType.String)
        {
            var jValue = (JValue)token;
            if (jValue.Value is string s)
            {
                // Remove null characters and trim whitespace
                jValue.Value = s.Replace("\u0000", "").Trim();
            }
            return token; // Return the modified string token
        }
        else if (token.Type == JTokenType.Null) // Convert null JTokens to empty strings for consistent hashing
        {
            return new JValue(string.Empty); // Return a new JValue representing an empty string
        }
        else if (token.Type == JTokenType.Object)
        {
            JObject obj = (JObject)token;
            foreach (var property in obj.Properties().ToList()) // Use ToList to allow modification during iteration
            {
                JToken cleanedValue = CleanJToken(property.Value);
                if (cleanedValue != property.Value) // If the value changed (e.g., null to empty string)
                {
                    property.Value = cleanedValue;
                }
            }
            return token; // Return the modified object token
        }
        else if (token.Type == JTokenType.Array)
        {
            JArray arr = (JArray)token;
            for (int i = 0; i < arr.Count; i++)
            {
                JToken cleanedItem = CleanJToken(arr[i]);
                if (cleanedItem != arr[i]) // If the item changed
                {
                    arr[i] = cleanedItem;
                }
            }
            return token; // Return the modified array token
        }
        return token; // Return token if no changes needed or other types
    }

    public dynamic Search(string index, string query)
    {
        var cleanBaseUrl = _baseUrl.TrimEnd('/');
        var url = $"{cleanBaseUrl}/{index}/_search";
        _logger.LogInformation("Search {url}", url);
        _logger.LogInformation("  query {query}", query);
        var content = new StringContent(query, Encoding.UTF8, "application/json");

        var response = _httpClient.PostAsync(url, content).Result;
        try {
            response.EnsureSuccessStatusCode();
            var json = response.Content.ReadAsStringAsync().Result;
            _logger.LogInformation("  response status: {status}", response.StatusCode);

            dynamic result = JsonConvert.DeserializeObject<dynamic>(json)!;
            _logger.LogInformation("  total hits: {total}", (int)result.hits.total);
            if (result.hits.hits.Count > 0)
            {
                _logger.LogInformation("  first document index: {index}", (string)result.hits.hits[0]._index);
                _logger.LogInformation("  first document ID: {id}", (string)result.hits.hits[0]._id);
            }
            _logger.LogDebug("  response {json}", json);
            return result;
        }
        catch (Exception ex)
        {
            _logger.LogError("Error searching {url}: {message}", url, ex.Message);
            throw;
        }
    }

    public string IndexDocument(string index, string id, object document)
    {
        var url = DeAliasURL(index, id);
        _logger.LogDebug("IndexDocumentAsync {url}", url);        
        var content = new StringContent(SerializeIgnoringNulls(document), Encoding.UTF8, "application/json");
        var response = _httpClient.PutAsync(url, content).Result;
        try {
            response.EnsureSuccessStatusCode();        
            return response.Content.ReadAsStringAsync().Result;
        }
        catch (Exception ex)
        {
            var errorResponse = response.Content.ReadAsStringAsync().Result;
            _logger.LogError("Error indexing document {id} to index {index}: {message}. Response: {response}", id, index, ex.Message, errorResponse);
            return "Error";
        }
    }

    public dynamic GetIndices()
    {
        var url = $"{_baseUrl}/_cat/indices?format=json";
        var response = _httpClient.GetAsync(url).Result;
        try {
            response.EnsureSuccessStatusCode();
            var json = response.Content.ReadAsStringAsync().Result;
            return JsonConvert.DeserializeObject<dynamic>(json)!;
        }
        catch (Exception ex)
        {
            _logger.LogError("Error getting indices: {message}", ex.Message);
            return "Error";
        }
    }

    public string DeleteDocument(string index, string id)
    {
        var url = DeAliasURL(index, id);        
        _logger.LogInformation("DeleteDocument {url}", url);
        var response = _httpClient.DeleteAsync(url).Result;
        try {
            response.EnsureSuccessStatusCode();
            return response.Content.ReadAsStringAsync().Result;
        }
        catch (Exception ex)
        {
            _logger.LogError("Error deleting document {id} from index {index}: {message}", id, index, ex.Message);
            return "Error";
        }
    }

    public string ConcreteIndex(string index)
    {
        if (IsAlias(index))
        {
            return GetAliasInfo(index)[0];
        }
        return index;
    }

    private string DeAliasURL(string index, string id)
    {
        var cleanBaseUrl = _baseUrl.TrimEnd('/');
        if (IsAlias(index))
        {
            var concreteIndex = ConcreteIndex(index);
            return $"{cleanBaseUrl}/{concreteIndex}/{index}/{id}";
        } 
        return $"{cleanBaseUrl}/{index}/{id}";
    }
    public bool IsAlias(string indexName)
    {
        try
        {
            var url = $"{_baseUrl}/_alias/{indexName}";
            var response = _httpClient.GetAsync(url).Result;            
            return response.IsSuccessStatusCode;
        }
        catch (Exception ex)
        {
            _logger.LogError("Error checking if {indexName} is an alias: {message}", indexName, ex.Message);
            return false;
        }
    }

    private List<string> GetAliasInfo(string aliasName)
    {
        try
        {
            var url = $"{_baseUrl}/_alias/{aliasName}";
            var response = _httpClient.GetAsync(url).Result;
            
            if (!response.IsSuccessStatusCode)
            {
                return new List<string>();
            }
            
            var json = response.Content.ReadAsStringAsync().Result;
            var aliasInfo = JsonConvert.DeserializeObject<Dictionary<string, object>>(json);
            
            if (aliasInfo == null || aliasInfo.Count == 0)
            {
                return new List<string>();
            }
            
            return aliasInfo.Keys.ToList();
        }
        catch (Exception ex)
        {
            _logger.LogError("Error getting alias info for {aliasName}: {message}", aliasName, ex.Message);
            return new List<string>();
        }
    }

    public string UpdateByQuery(string index, string query)
    {
        var url = $"{_baseUrl}/{index}/_update_by_query";
        _logger.LogInformation("UpdateByQuery {url}", url);
        var content = new StringContent(query, Encoding.UTF8, "application/json");
        
        var response = _httpClient.PostAsync(url, content).Result;
        try {
            response.EnsureSuccessStatusCode();        
            return response.Content.ReadAsStringAsync().Result;
        }
        catch (Exception ex)
        {
            _logger.LogError("Error updating document by query {query}: {message}", query, ex.Message);
            return "Error";
        }
    }

        public static List<string> GetIdsFromDynamic(dynamic searchResults)
    {
        var ids = new List<string>();
        foreach (var hit in searchResults.hits.hits)
        {
            ids.Add(hit._id.ToString());
        }
        return ids;
    }
    public static string SerializeKeepingNulls(object obj)
    {
        var settings = new JsonSerializerSettings { NullValueHandling = NullValueHandling.Include };
        return JsonConvert.SerializeObject(obj, settings);
    }
    public static string SerializeIgnoringNulls(object obj)
    {
        var settings = new JsonSerializerSettings { NullValueHandling = NullValueHandling.Ignore };
        return JsonConvert.SerializeObject(obj, settings);
    }
    public static string BulkToJson(string index, string type, IEnumerable<object> docs)
    {
        var settings = new JsonSerializerSettings { NullValueHandling = NullValueHandling.Ignore };
        
        var json = new StringBuilder();
        foreach (var doc in docs)
        {
            var meta = new { index = new { _index = index, _type = type } };
            json.AppendLine(SerializeKeepingNulls(meta));
            json.AppendLine(SerializeIgnoringNulls(doc));
        }
        return json.ToString();
    }
    public string BulkIndex(string index, IEnumerable<object> docs)
    {
        var url = DeAliasURL(index, "_bulk");
        var json = BulkToJson(ConcreteIndex(index), index, docs);
        _logger.LogDebug("BulkIndex {json}", json);
        var content = new StringContent(json, Encoding.UTF8, "application/json");
        var response = _httpClient.PostAsync(url, content).Result;
        try {
            response.EnsureSuccessStatusCode();
            return response.Content.ReadAsStringAsync().Result;
        }
        catch (Exception ex)
        {
            var errorResponse = response.Content.ReadAsStringAsync().Result;
            _logger.LogError("Error bulk indexing documents: {message}. Response: {response}", ex.Message, errorResponse);
            return "Error";
        }
    }

    public string BulkDelete(string index, IEnumerable<string> ids)
    {
        foreach (var id in ids)
        {
            DeleteDocument(index, id);
        }
        return "Success";
    }
}