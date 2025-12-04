# SharePoint - SharePoint Online Integration Patterns

This skill provides proven patterns for integrating with SharePoint Online using both CSOM (Client-Side Object Model) and Microsoft Graph API approaches.

## Authentication Configuration

SharePoint authentication requires Azure AD App Registration with appropriate permissions.

### Configuration File Format

```json
{
    "TenantId": "your-tenant-id",
    "ClientId": "your-client-id",
    "ClientSecret": "your-client-secret"
}
```

### Required Azure AD Permissions

**For CSOM (PnP.Framework)**:
- SharePoint: `Sites.FullControl.All` (Application permission)
- Uses App-Only authentication with Client ID/Secret

**For Graph API**:
- Microsoft Graph: `Sites.ReadWrite.All` (Application permission)
- Microsoft Graph: `Files.ReadWrite.All` (Application permission)

## Two Approaches: CSOM vs Graph API

### When to Use CSOM (PnP.Framework)
- File uploads (especially chunked uploads for large files)
- Folder operations (create, navigate, check existence)
- Bulk file operations
- Better performance for file-heavy operations
- Server-relative URL navigation

### When to Use Graph API
- Excel file operations (reading/writing cells, tables)
- Modern SharePoint features
- Cross-Microsoft 365 integration
- REST-based operations
- When you need JSON responses

## Pattern 1: CSOM File Upload with Chunking

```csharp
using Microsoft.SharePoint.Client;
using PnP.Framework;
using Microsoft.Extensions.Logging;

public class Sharepoint
{
    private readonly ClientContext ctx;
    private readonly ILogger _logger;

    public Sharepoint(string host, string site, Dictionary<string, string> config, ILogger logger)
    {
        var authManager = new AuthenticationManager();
        ctx = authManager.GetACSAppOnlyContext(
            $"https://{host}/sites/{site}",
            config["ClientId"],
            config["ClientSecret"]
        );
        _logger = logger;
    }

    /// <summary>
    /// Uploads a file to SharePoint with automatic chunking for large files
    /// Files > 1MB use chunked upload, files > 50MB are rejected
    /// </summary>
    public string UploadImage(Folder folder, string localPath)
    {
        var fileInfo = new FileInfo(localPath);
        var fileName = Path.GetFileName(localPath);

        ctx.Load(folder, f => f.ServerRelativeUrl);
        ctx.ExecuteQuery();

        _logger.LogInformation($"Uploading file: {fileName} ({fileInfo.Length / 1024.0:F2} KB)");

        // If file is larger than 1MB, use chunked upload
        if (fileInfo.Length >= 1 * 1024 * 1024)
        {
            return UploadLargeImage(folder, localPath);
        }

        // For smaller files, use simple upload
        var fileBytes = File.ReadAllBytes(localPath);
        var fileCreationInfo = new FileCreationInformation
        {
            Content = fileBytes,
            Url = fileName,
            Overwrite = true
        };

        var uploadedFile = folder.Files.Add(fileCreationInfo);
        ctx.Load(uploadedFile, f => f.ServerRelativeUrl);
        ctx.ExecuteQuery();

        _logger.LogInformation($"Successfully uploaded {fileName} to {uploadedFile.ServerRelativeUrl}");
        return uploadedFile.ServerRelativeUrl;
    }

    /// <summary>
    /// Chunked upload for files between 1MB and 50MB
    /// </summary>
    private string UploadLargeImage(Folder folder, string localPath)
    {
        var fileName = Path.GetFileName(localPath);
        var fileInfo = new FileInfo(localPath);

        if (fileInfo.Length > 50 * 1024 * 1024)
        {
            throw new InvalidOperationException(
                $"File {fileName} is too large ({fileInfo.Length / (1024.0 * 1024.0):F2} MB). " +
                "Files larger than 50MB cannot be uploaded with this method.");
        }

        // Step 1: Create empty file placeholder
        var emptyContentStream = new MemoryStream();
        var fileCreationInfo = new FileCreationInformation
        {
            ContentStream = emptyContentStream,
            Url = fileName,
            Overwrite = true
        };

        var uploadFile = folder.Files.Add(fileCreationInfo);
        ctx.Load(uploadFile, f => f.ServerRelativeUrl);
        ctx.ExecuteQuery();

        // Step 2: Upload in 1MB chunks
        const int chunkSize = 1024 * 1024;
        var uploadId = Guid.NewGuid();
        long fileOffset = 0;

        using (var fileStream = File.OpenRead(localPath))
        {
            var buffer = new byte[chunkSize];
            int bytesRead;
            bool isFirstChunk = true;

            while ((bytesRead = fileStream.Read(buffer, 0, chunkSize)) > 0)
            {
                var chunkStream = new MemoryStream();
                chunkStream.Write(buffer, 0, bytesRead);
                chunkStream.Position = 0;

                if (isFirstChunk)
                {
                    var bytesUploaded = uploadFile.StartUpload(uploadId, chunkStream);
                    ctx.ExecuteQuery();
                    fileOffset = bytesUploaded.Value;
                    isFirstChunk = false;
                }
                else if (fileStream.Position >= fileStream.Length)
                {
                    uploadFile.FinishUpload(uploadId, fileOffset, chunkStream);
                    ctx.ExecuteQuery();
                    break;
                }
                else
                {
                    var bytesUploaded = uploadFile.ContinueUpload(uploadId, fileOffset, chunkStream);
                    ctx.ExecuteQuery();
                    fileOffset = bytesUploaded.Value;
                }

                chunkStream.Dispose();
            }
        }

        return uploadFile.ServerRelativeUrl;
    }
}
```

**Usage:**
```csharp
var logger = Utf8LoggingExtensions.CreateUtf8Logger("MyApp", LogLevel.Information);
var config = DictionaryLoader.LoadDictionary("azure_ids.json");
var sp = new Sharepoint("tenant.sharepoint.com", "MySite", config, logger);

// Get or create folder
var folder = sp.GetOrCreateFolderByServerRelativeUrl("/sites/MySite/Documents/TargetFolder");

// Upload file (automatically uses chunking if needed)
var uploadedUrl = sp.UploadImage(folder, "C:\\path\\to\\large-file.jpg");
```

## Pattern 2: Folder Management with CSOM

```csharp
/// <summary>
/// Gets an existing folder or creates the entire folder hierarchy
/// </summary>
public Folder GetOrCreateFolderByServerRelativeUrl(string serverRelativeUrl)
{
    try
    {
        // Try to get existing folder
        var folder = ctx.Web.GetFolderByServerRelativeUrl(serverRelativeUrl);
        ctx.Load(folder, f => f.Exists);
        ctx.ExecuteQuery();

        if (folder.Exists)
        {
            return folder;
        }
    }
    catch (ServerException ex) when (ex.ServerErrorTypeName == "System.IO.FileNotFoundException")
    {
        _logger.LogInformation($"Folder doesn't exist at {serverRelativeUrl}, creating it...");
    }

    // Create folder hierarchy level by level
    string[] parts = serverRelativeUrl.Split('/', StringSplitOptions.RemoveEmptyEntries);
    string currentPath = "";
    Folder currentFolder = ctx.Web.RootFolder;

    foreach (string part in parts)
    {
        currentPath = string.IsNullOrEmpty(currentPath) ? "/" + part : currentPath + "/" + part;

        try
        {
            currentFolder = ctx.Web.GetFolderByServerRelativeUrl(currentPath);
            ctx.Load(currentFolder, f => f.Exists);
            ctx.ExecuteQuery();

            if (!currentFolder.Exists)
            {
                throw new Exception("Folder doesn't exist");
            }
        }
        catch
        {
            _logger.LogInformation($"Creating folder: {currentPath}");
            currentFolder = currentFolder.Folders.Add(part);
            ctx.Load(currentFolder);
            ctx.ExecuteQuery();
        }
    }

    return currentFolder;
}

/// <summary>
/// Checks if a file exists in a folder
/// </summary>
public bool FileExists(Folder folder, string fileName)
{
    try
    {
        ctx.Load(folder, f => f.ServerRelativeUrl);
        ctx.ExecuteQuery();

        var serverRelativeUrl = $"{folder.ServerRelativeUrl}/{fileName}";
        var file = ctx.Web.GetFileByServerRelativeUrl(serverRelativeUrl);
        ctx.Load(file, f => f.Exists);
        ctx.ExecuteQuery();

        return file.Exists;
    }
    catch (ServerException ex) when (ex.ServerErrorTypeName == "System.IO.FileNotFoundException")
    {
        return false;
    }
}
```

**Usage:**
```csharp
// Create nested folder structure
var folder = sp.GetOrCreateFolderByServerRelativeUrl(
    "/sites/RIHub/External Partners/Customer Service/ClientName/2025 Orders");

// Check if file exists before uploading
if (!sp.FileExists(folder, "image.jpg"))
{
    sp.UploadImage(folder, localFilePath);
}
```

## Pattern 3: Excel Operations with Graph API

```csharp
using Microsoft.Graph;
using Azure.Identity;
using Microsoft.Graph.Models;
using System.Text.Json;

public class SharepointSite
{
    private readonly GraphServiceClient _graphServiceClient;
    private readonly Site? _site;
    private readonly ClientSecretCredential _credential;
    private readonly ILogger _logger;

    public SharepointSite(string host, string site, Dictionary<string, string> config, ILogger logger)
    {
        _logger = logger;
        _credential = new ClientSecretCredential(
            config["TenantId"],
            config["ClientId"],
            config["ClientSecret"]);

        _graphServiceClient = new GraphServiceClient(_credential);
        _site = _graphServiceClient.Sites[$"{host}:/sites/{site}"].GetAsync().Result;
    }

    /// <summary>
    /// Updates a single cell in an Excel file
    /// </summary>
    public async Task UpdateExcelCell(string driveName, string filePath,
        string sheetName, string cellAddress, string newValue)
    {
        var drive = GetDrive(driveName);
        var driveItem = await GetFileForUpdate(driveName, filePath);

        var updateUrl = $"https://graph.microsoft.com/v1.0/drives/{drive.Id}/" +
                       $"items/{driveItem.Id}/workbook/worksheets/{sheetName}/" +
                       $"range(address='{cellAddress}')";

        var jsonContent = JsonSerializer.Serialize(new {
            values = new[] { new[] { newValue } }
        });

        await SendGraphRequest(HttpMethod.Patch, updateUrl, jsonContent);
    }

    private Drive GetDrive(string name)
    {
        var driveId = GetDriveID(name);
        var drive = _graphServiceClient.Sites[_site.Id]
            .Drives[driveId]
            .GetAsync(requestConfiguration => {
                requestConfiguration.QueryParameters.Select = new[] {
                    "id", "name", "driveType", "root", "webUrl"
                };
                requestConfiguration.QueryParameters.Expand = new[] { "root" };
            })
            .GetAwaiter()
            .GetResult();

        return drive;
    }
}
```

**Usage:**
```csharp
var spSite = new SharepointSite("tenant.sharepoint.com", "TradingDepartment", config, logger);

// Update cell N123 to "Y" in "Repro Tracker" sheet
await spSite.UpdateExcelCell(
    "Trading",                                      // Drive name
    "SEASONAL/EASTER/Easter 2025/tracker.xlsx",   // File path
    "Repro Tracker",                              // Sheet name
    "N123",                                       // Cell address
    "Y"                                           // New value
);
```

## Pattern 4: Download Files from SharePoint

```csharp
/// <summary>
/// Downloads a file from SharePoint as a MemoryStream
/// </summary>
public MemoryStream GetFileFromRelativeUrl(string serverRelativeUrl)
{
    var file = ctx.Web.GetFileByServerRelativeUrl(serverRelativeUrl);
    var streamResult = file.OpenBinaryStream();
    ctx.ExecuteQuery();

    var stream = streamResult.Value ?? throw new InvalidOperationException("File stream is null");
    var ms = new MemoryStream();
    stream.CopyTo(ms);
    ms.Position = 0; // Reset to beginning for reading

    return ms;
}
```

**Usage:**
```csharp
// Download Excel file for processing
var ms = sp.GetFileFromRelativeUrl(
    "/sites/TradingDepartment/Trading/SEASONAL/EASTER/tracker.xlsx");

// Process with Aspose.Cells
var workbook = new Aspose.Cells.Workbook(ms);
```

## Pattern 5: SharePoint URL Normalization

SharePoint URLs come in many formats. Here's how to normalize them:

```csharp
public string FixSharePointLink(string rawLink)
{
    string sharepointHost = "https://tenant.sharepoint.com";

    // Remove query parameters
    string cleanLink = rawLink;
    int queryParamStart = cleanLink.IndexOf('?');
    if (queryParamStart != -1)
    {
        cleanLink = cleanLink.Substring(0, queryParamStart);
    }

    // Handle :f:/r/ redirect format (SharePoint sharing links)
    int redirectMarkerIndex = cleanLink.LastIndexOf(":f:/r/", StringComparison.OrdinalIgnoreCase);
    if (redirectMarkerIndex != -1)
    {
        string pathAfterMarker = cleanLink.Substring(redirectMarkerIndex + ":f:/r/".Length);
        string decodedPath = Uri.UnescapeDataString(pathAfterMarker);

        if (!decodedPath.StartsWith("sites/", StringComparison.OrdinalIgnoreCase))
        {
            decodedPath = "sites/" + decodedPath;
        }

        return $"{sharepointHost}/{decodedPath}";
    }

    // Handle Forms/AllItems.aspx format with id parameter
    if (rawLink.Contains("/Forms/AllItems.aspx", StringComparison.OrdinalIgnoreCase))
    {
        var queryParams = rawLink.Replace("?", "&").Split('&');
        var idParam = queryParams.FirstOrDefault(p =>
            p.StartsWith("id=", StringComparison.OrdinalIgnoreCase));

        if (idParam != null)
        {
            var encodedPath = idParam.Substring(3);
            var decodedPath = Uri.UnescapeDataString(encodedPath);
            return $"{sharepointHost}{decodedPath}";
        }
    }

    // Direct URL - just decode
    if (Uri.TryCreate(rawLink, UriKind.Absolute, out Uri? uri))
    {
        string absolutePath = Uri.UnescapeDataString(uri.AbsolutePath);
        return $"{sharepointHost}{absolutePath}";
    }

    return string.Empty;
}
```

## Required NuGet Packages

```xml
<PackageReference Include="PnP.Framework" Version="1.17.0" />
<PackageReference Include="Microsoft.Graph" Version="5.70.0" />
<PackageReference Include="Microsoft.Identity.Client" Version="4.67.2" />
<PackageReference Include="Azure.Identity" Version="1.13.2" />
<PackageReference Include="Microsoft.Extensions.Logging.Abstractions" Version="9.0.8" />
```

## Common Patterns

### Error Handling

```csharp
try
{
    var folder = sp.GetOrCreateFolderByServerRelativeUrl(folderPath);
    sp.UploadImage(folder, localPath);
}
catch (Microsoft.SharePoint.Client.ServerUnauthorizedAccessException ex)
{
    _logger.LogError(ex, "Access denied. Check SharePoint permissions.");
}
catch (InvalidOperationException ex) when (ex.Message.Contains("too large"))
{
    _logger.LogError(ex, "File is too large for upload.");
}
catch (Exception ex)
{
    _logger.LogError(ex, "Unexpected error during SharePoint operation.");
}
```

### Performance Tips

1. **Batch operations** - CSOM supports ExecuteQueryRetry() for resilience
2. **Use chunked uploads** for files > 1MB
3. **Cache Drive IDs** when doing multiple Graph API operations
4. **Minimize ExecuteQuery() calls** - load multiple objects before executing
5. **Use server-relative URLs** for better performance with CSOM

### Authentication Best Practices

1. Store credentials in **external JSON files**, never in code
2. Use **Azure Key Vault** for production environments
3. Set appropriate **token lifetimes** (default is usually fine)
4. Monitor for **authentication failures** and handle token refresh
5. **Rotate client secrets** before expiration (check Azure AD portal)

## Troubleshooting

### "Access Denied" Errors
- Check Azure AD app permissions in Azure Portal
- Ensure app has been granted admin consent
- Verify site collection administrators if using CSOM
- Check if the folder/file has unique permissions

### "File Not Found" Errors
- Verify server-relative URLs start with `/sites/`
- Check URL encoding - use `Uri.UnescapeDataString()`
- Ensure folder exists before uploading files
- Use `GetOrCreateFolderByServerRelativeUrl()` to create missing folders

### Chunked Upload Failures
- Verify file is not > 50MB (hard limit in this implementation)
- Check network stability for large uploads
- Consider implementing retry logic with exponential backoff
- Monitor `uploadId` - each upload session needs unique GUID

### Graph API Excel Errors
- Ensure Excel file is **not open** in browser during updates
- Check that **worksheet name** matches exactly (case-sensitive)
- Verify cell address format (e.g., "A1", "N123")
- Use **server-relative paths** for file paths in Graph API

## Real-World Example: Image Upload Pipeline

```csharp
public async Task UploadSeasonalImages(
    string imagePath,
    string trackerPath,
    Dictionary<string, string> config)
{
    var logger = Utf8LoggingExtensions.CreateUtf8Logger("ImageUploader", LogLevel.Information);

    // Initialize both CSOM and Graph API clients
    var rihub = new Sharepoint("tenant.sharepoint.com", "RIHub", config, logger);
    var td = new Sharepoint("tenant.sharepoint.com", "TradingDepartment", config, logger);
    var tdSite = new SharepointSite("tenant.sharepoint.com", "TradingDepartment", config, logger);

    // Get tracker Excel file
    var trackerMs = td.GetFileFromRelativeUrl(trackerPath);
    var tracker = new TrackerProcessor(trackerMs, logger);

    var rows = tracker.GetRows();

    foreach (var row in rows)
    {
        // Find local images
        var localImages = Directory.GetFiles(imagePath, $"{row.Barcode}*.*",
            SearchOption.AllDirectories);

        if (localImages.Length == 0)
        {
            logger.LogWarning($"No images found for barcode {row.Barcode}");
            continue;
        }

        // Get/create SharePoint folder
        var spFolder = rihub.GetOrCreateFolderByServerRelativeUrl(row.SharePointFolderPath);

        foreach (var imagePath in localImages)
        {
            try
            {
                // Check if already exists
                var fileName = Path.GetFileName(imagePath);
                if (!rihub.FileExists(spFolder, fileName))
                {
                    // Upload (automatically handles chunking)
                    var uploadedUrl = rihub.UploadImage(spFolder, imagePath);
                    logger.LogInformation($"Uploaded {fileName} to {uploadedUrl}");

                    // Mark as complete in Excel tracker
                    await tdSite.UpdateExcelCell(
                        "Trading",
                        trackerPath,
                        "Repro Tracker",
                        $"N{row.RowNumber}",
                        "Y");
                }
                else
                {
                    logger.LogInformation($"File {fileName} already exists, skipping");
                }
            }
            catch (Exception ex)
            {
                logger.LogError(ex, $"Failed to upload {imagePath}");
            }
        }
    }
}
```

## Notes

- **CSOM is faster** for file operations but has older API
- **Graph API is modern** but can be slower for bulk operations
- Consider **hybrid approach**: CSOM for files, Graph for Excel
- Always **log operations** - SharePoint operations can be slow
- Implement **retry logic** for production environments
- **Test with small files first** before uploading large batches

## References

- [PnP Framework Documentation](https://pnp.github.io/pnpframework/)
- [Microsoft Graph API - Sites](https://learn.microsoft.com/en-us/graph/api/resources/sharepoint)
- [SharePoint CSOM Reference](https://learn.microsoft.com/en-us/sharepoint/dev/sp-add-ins/complete-basic-operations-using-sharepoint-client-library-code)
- [Azure AD App Registration](https://learn.microsoft.com/en-us/azure/active-directory/develop/quickstart-register-app)
