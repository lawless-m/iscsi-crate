# Email - SMTP Email with Zipped Log Attachments

This skill provides patterns for sending emails via SMTP with automatic log file compression and multiple attachment handling.

## The Problem

Sending large log files as email attachments causes issues:
- **Email size limits** - Most SMTP servers reject messages > 25-50MB
- **Log files can be huge** - Daily logs with verbose output grow quickly
- **File locking issues** - Log files are often locked by running processes
- **Disk cleanup** - Temporary zip files need cleanup after sending

## The Solution

A battle-tested pattern that:
1. Reads locked log files using `FileShare.ReadWrite`
2. Compresses logs into temporary zip files
3. Attaches both zipped logs and additional files
4. Cleans up temporary files in `finally` blocks
5. Falls back gracefully on errors

## Pattern 1: Basic Email Sending

```csharp
using System.Net.Mail;

public class Email
{
    private readonly string _to;
    private readonly string _subject;
    private readonly string _body;

    public Email(string to, string subject, string body)
    {
        _to = to;
        _subject = subject;
        _body = body;
    }

    public bool Send(string? from = null)
    {
        try
        {
            var smtpClient = new SmtpClient("smtp.nisainternational.local");
            var mailMessage = new MailMessage(
                from ?? "services@ramsden-international.com",
                _to,
                _subject,
                _body
            );
            smtpClient.Send(mailMessage);
            return true;
        }
        catch (Exception ex)
        {
            Console.WriteLine(ex.Message);
            return false;
        }
    }
}
```

**Usage:**
```csharp
var email = new Email(
    "recipient@company.com",
    "Daily Report",
    "Processing completed successfully"
);
email.Send();
```

## Pattern 2: Email with Simple Attachments

```csharp
public bool Send(string? from, string? attachmentPath)
{
    try
    {
        var smtpClient = new SmtpClient("smtp.nisainternational.local");
        using var mailMessage = new MailMessage(  // ← IMPORTANT: using statement for disposal
            from ?? "services@ramsden-international.com",
            _to,
            _subject,
            _body
        );

        if (!string.IsNullOrEmpty(attachmentPath) && File.Exists(attachmentPath))
        {
            mailMessage.Attachments.Add(new Attachment(attachmentPath));  // ← Create inline
        }

        smtpClient.Send(mailMessage);
        return true;
    }
    catch (Exception ex)
    {
        Console.WriteLine(ex.Message);
        return false;
    }
}
```

## Pattern 3: Zipped Log Attachments (The Key Pattern!)

This is the important one - handles locked files and compression:

```csharp
using System.IO.Compression;

public bool SendWithZippedAttachment(string? from, string? attachmentPath, bool deleteOriginal = false)
{
    if (string.IsNullOrEmpty(attachmentPath))
    {
        Console.WriteLine("No attachment path provided, sending without attachment");
        return Send(from);
    }

    if (!File.Exists(attachmentPath))
    {
        Console.WriteLine($"Attachment file not found: {attachmentPath}, sending without attachment");
        return Send(from);
    }

    string? zippedPath = null;
    bool emailSent = false;

    try
    {
        Console.WriteLine($"Creating zip for {attachmentPath}");
        zippedPath = ZipSingleFileFromStream(attachmentPath);
        Console.WriteLine($"Zip created at {zippedPath}");

        emailSent = Send(from, zippedPath);
        Console.WriteLine($"Email sent with zip attachment: {emailSent}");

        if (deleteOriginal && File.Exists(attachmentPath))
        {
            File.Delete(attachmentPath);
            Console.WriteLine($"Original file deleted: {attachmentPath}");
        }

        return emailSent;
    }
    catch (Exception ex)
    {
        Console.WriteLine($"Error zipping attachment: {ex.Message}");
        Console.WriteLine($"Falling back to original file attachment");
        return Send(from, attachmentPath);
    }
    finally
    {
        // CRITICAL: Clean up zip file after email is sent
        if (!string.IsNullOrEmpty(zippedPath) && File.Exists(zippedPath))
        {
            try
            {
                File.Delete(zippedPath);
                Console.WriteLine($"Zip file cleaned up: {zippedPath}");
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Failed to clean up zip file: {ex.Message}");
            }
        }
    }
}

/// <summary>
/// The SECRET SAUCE: Opens file with FileShare.ReadWrite to read locked log files
/// </summary>
public static string ZipSingleFileFromStream(string filePath, string? outputPath = null)
{
    if (!File.Exists(filePath))
    {
        throw new FileNotFoundException($"File not found: {filePath}");
    }

    string zipPath = outputPath ?? Path.ChangeExtension(filePath, ".zip");

    if (File.Exists(zipPath))
    {
        File.Delete(zipPath);
    }

    using (var zipArchive = ZipFile.Open(zipPath, ZipArchiveMode.Create))
    {
        var entry = zipArchive.CreateEntry(Path.GetFileName(filePath), CompressionLevel.Optimal);

        using (var entryStream = entry.Open())
        using (var fileStream = new FileStream(
            filePath,
            FileMode.Open,
            FileAccess.Read,
            FileShare.ReadWrite))  // ← THIS IS KEY! Allows reading locked files
        {
            fileStream.CopyTo(entryStream);
        }
    }

    return zipPath;
}
```

**Why FileShare.ReadWrite matters:**
- Log files are often **locked by running processes** (logging frameworks keep them open)
- `FileShare.ReadWrite` tells Windows "I want to read this file even if another process has it open for writing"
- Without this, you get `IOException: The process cannot access the file because it is being used by another process`

## Pattern 4: Multiple Attachments (Zipped Log + Additional Files)

For reports that need both compressed logs AND supplemental files:

```csharp
public bool SendWithZippedAttachmentAndAdditionalFiles(
    string? from,
    string? logFileToZip,
    bool deleteOriginal = false,
    params string[] additionalAttachments)
{
    string? zippedPath = null;
    bool emailSent = false;

    try
    {
        var smtpClient = new SmtpClient("smtp.nisainternational.local");
        using var mailMessage = new MailMessage(  // ← CRITICAL: using statement for disposal!
            from ?? "services@ramsden-international.com",
            _to,
            _subject,
            _body
        );

        // Add zipped log file if provided
        if (!string.IsNullOrEmpty(logFileToZip) && File.Exists(logFileToZip))
        {
            Console.WriteLine($"Creating zip for {logFileToZip}");
            zippedPath = ZipSingleFileFromStream(logFileToZip);
            Console.WriteLine($"Zip created at {zippedPath}");

            if (File.Exists(zippedPath))
            {
                mailMessage.Attachments.Add(new Attachment(zippedPath));  // ← Create inline
            }

            if (deleteOriginal)
            {
                File.Delete(logFileToZip);
                Console.WriteLine($"Original file deleted: {logFileToZip}");
            }
        }

        // Add additional attachments (e.g., CSV files, reports)
        foreach (var attachmentPath in additionalAttachments)
        {
            if (!string.IsNullOrEmpty(attachmentPath) && File.Exists(attachmentPath))
            {
                mailMessage.Attachments.Add(new Attachment(attachmentPath));  // ← Create inline
                Console.WriteLine($"Added additional attachment: {attachmentPath}");
            }
        }

        smtpClient.Send(mailMessage);
        emailSent = true;
        Console.WriteLine($"Email sent successfully");
        return true;
    }
    catch (Exception ex)
    {
        Console.WriteLine($"Error sending email: {ex.Message}");
        return false;
    }
    finally
    {
        // Clean up zip file after email is sent
        // Note: With using statement above, file handles are released before we get here
        if (!string.IsNullOrEmpty(zippedPath) && File.Exists(zippedPath))
        {
            try
            {
                File.Delete(zippedPath);
                Console.WriteLine($"Zip file cleaned up: {zippedPath}");
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Failed to clean up zip file: {ex.Message}");
            }
        }
    }
}
```

**Usage:**
```csharp
var email = new Email(
    "manager@company.com",
    "Daily Processing Report",
    "See attached logs and missing items report"
);

// Send with zipped log + CSV of missing barcodes
email.SendWithZippedAttachmentAndAdditionalFiles(
    null,                                           // from (use default)
    "C:\\Logs\\app_2025_11_03.log",                // log to zip
    false,                                          // don't delete original
    "C:\\Reports\\MissingBarcodes_2025_11_03.txt"  // additional file
);
```

## Real-World Example: Daily Report with Logs

```csharp
public void SendDailyReport(List<string> processingMessages, string logFilePath)
{
    // Generate summary
    int totalProcessed = processingMessages.Count(m => m.Contains("SUCCESS"));
    int errors = processingMessages.Count(m => m.Contains("ERROR"));

    var summary = $@"Daily Processing Report
========================

Total Items Processed: {totalProcessed}
Errors Encountered: {errors}

For detailed logs, see attached file.

Execution completed at: {DateTime.Now:yyyy-MM-dd HH:mm:ss}";

    // Create missing items file if needed
    var missingItems = processingMessages
        .Where(m => m.Contains("No images found"))
        .ToList();

    string? missingItemsPath = null;
    if (missingItems.Any())
    {
        missingItemsPath = Path.Combine(
            Path.GetDirectoryName(logFilePath) ?? "C:\\Logs",
            $"MissingItems_{DateTime.Now:yyyy_MM_dd}.txt"
        );

        File.WriteAllText(missingItemsPath, string.Join("\n", missingItems));
        Console.WriteLine($"Created missing items file: {missingItemsPath}");
    }

    // Send email with zipped log + missing items file
    var email = new Email(
        "operations@company.com",
        "Daily Processing Report",
        summary
    );

    if (!string.IsNullOrEmpty(missingItemsPath))
    {
        email.SendWithZippedAttachmentAndAdditionalFiles(
            null,
            logFilePath,
            false,
            missingItemsPath
        );

        // Clean up missing items file after sending
        try
        {
            File.Delete(missingItemsPath);
            Console.WriteLine($"Cleaned up: {missingItemsPath}");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Failed to clean up: {ex.Message}");
        }
    }
    else
    {
        email.SendWithZippedAttachment(null, logFilePath, false);
    }
}
```

## Configuration for Different Environments

### Development Environment
```csharp
// Use local pickup directory for testing (no actual SMTP)
var smtpClient = new SmtpClient
{
    DeliveryMethod = SmtpDeliveryMethod.SpecifiedPickupDirectory,
    PickupDirectoryLocation = @"C:\EmailPickup"
};
```

### Production Environment
```csharp
// Standard SMTP server
var smtpClient = new SmtpClient("smtp.nisainternational.local");
```

### External SMTP (Gmail, Office 365, etc.)
```csharp
var smtpClient = new SmtpClient("smtp.gmail.com")
{
    Port = 587,
    Credentials = new NetworkCredential("user@gmail.com", "app-password"),
    EnableSsl = true
};
```

## Common Pitfalls and Solutions

### Pitfall 1: Forgetting to Clean Up Zip Files
**Problem:** Disk fills up with temporary .zip files
**Solution:** Always use `finally` blocks to delete temporary files

```csharp
finally
{
    if (!string.IsNullOrEmpty(zippedPath) && File.Exists(zippedPath))
    {
        File.Delete(zippedPath);
    }
}
```

### Pitfall 2: File Locking Issues
**Problem:** `IOException: The process cannot access the file`
**Solution:** Use `FileShare.ReadWrite` when opening files

```csharp
using (var fileStream = new FileStream(
    filePath,
    FileMode.Open,
    FileAccess.Read,
    FileShare.ReadWrite))  // ← Allows reading locked files
```

### Pitfall 3: Large Attachment Failures
**Problem:** Email server rejects messages > 25MB
**Solution:** Compression helps, but also check limits

```csharp
var fileInfo = new FileInfo(zipPath);
if (fileInfo.Length > 25 * 1024 * 1024)  // 25MB
{
    Console.WriteLine($"Warning: Attachment is {fileInfo.Length / (1024.0 * 1024.0):F2}MB");
    // Consider splitting into multiple emails or using cloud storage links
}
```

### Pitfall 4: SMTP Authentication Errors
**Problem:** Email fails silently with no clear error
**Solution:** Add detailed logging

```csharp
try
{
    Console.WriteLine($"Connecting to SMTP server...");
    smtpClient.Send(mailMessage);
    Console.WriteLine($"Email sent successfully to {_to}");
}
catch (SmtpException ex)
{
    Console.WriteLine($"SMTP Error: {ex.StatusCode} - {ex.Message}");
    throw;
}
```

### Pitfall 5: Undisposed MailMessage and Attachment Objects (CRITICAL!)
**Problem:** Files remain locked after email is sent, even after 10+ minutes
**Root Cause:** `MailMessage` and `Attachment` objects hold file handles that aren't released until garbage collection
**Symptom:** `IOException: The process cannot access the file because it is being used by another process` when trying to delete temp files

**WRONG WAY (leaks file handles):**
```csharp
var mailMessage = new MailMessage(from, to, subject, body);

if (File.Exists(attachmentPath))
{
    var attachment = new Attachment(attachmentPath);  // Opens file handle
    mailMessage.Attachments.Add(attachment);
}

smtpClient.Send(mailMessage);
// File handles still open! Won't be released until GC runs
```

**RIGHT WAY (properly disposes):**
```csharp
using var mailMessage = new MailMessage(from, to, subject, body);  // ← using statement!

if (File.Exists(attachmentPath))
{
    mailMessage.Attachments.Add(new Attachment(attachmentPath));  // ← Create inline, no variable
}

smtpClient.Send(mailMessage);
// When using block exits, mailMessage.Dispose() is called
// This automatically disposes all attachments in the collection
// File handles are immediately released
```

**Why This Matters:**
1. `Attachment` objects open file handles when created
2. Without `using`, those handles stay open indefinitely (until GC runs)
3. You can't delete files while handles are open
4. Even with retry logic (5 attempts × 60 seconds), files stay locked
5. `MailMessage.Dispose()` automatically disposes all attachments in its `Attachments` collection

**Key Principle:**
- Create `Attachment` objects **inline** (don't store in variables)
- Let `MailMessage.Attachments` collection own them
- Use `using` on `MailMessage` to ensure disposal
- When `MailMessage` is disposed, all attachments are disposed too

## Required NuGet Packages

None! This uses built-in .NET libraries:
- `System.Net.Mail` (SMTP)
- `System.IO.Compression` (Zipping)

## Performance Tips

1. **Compression is fast** - Don't skip it even for "small" files (text compresses well)
2. **Use async for large batches** - If sending 10+ emails, consider `SendAsync()`
3. **Reuse SmtpClient carefully** - It's not thread-safe, use one per thread
4. **Dispose properly** - `MailMessage` and `Attachment` objects hold file handles

## Integration with UTF-8 Logging

```csharp
// Get current log file path from UTF8Writer
var logPath = UTF8Writer.GetCurrentLogFilePath();

// Send daily report with log attached
var email = new Email(
    "ops@company.com",
    "Daily Report",
    GenerateReportSummary()
);

email.SendWithZippedAttachment(null, logPath, deleteOriginal: false);
```

## Testing Strategy

```csharp
// 1. Test basic sending
var email = new Email("test@company.com", "Test", "Body");
Assert.True(email.Send());

// 2. Test with attachment
var testFile = "test.txt";
File.WriteAllText(testFile, "Test content");
Assert.True(email.Send(null, testFile));

// 3. Test with zipped attachment
var largeTestFile = "test.log";
File.WriteAllText(largeTestFile, new string('x', 10_000_000)); // 10MB
Assert.True(email.SendWithZippedAttachment(null, largeTestFile));
Assert.False(File.Exists(Path.ChangeExtension(largeTestFile, ".zip"))); // Verify cleanup

// 4. Test with locked file
using (var fs = File.Open(largeTestFile, FileMode.Open, FileAccess.Write, FileShare.Read))
{
    // File is locked by this process
    Assert.True(email.SendWithZippedAttachment(null, largeTestFile)); // Should still work!
}
```

## When to Use Each Method

| Method | Use Case |
|--------|----------|
| `Send()` | Simple notifications, no attachments |
| `Send(from, attachmentPath)` | Small files (< 5MB), not actively locked |
| `SendWithZippedAttachment()` | **Log files, large text files, locked files** |
| `SendWithZippedAttachmentAndAdditionalFiles()` | **Daily reports with logs + CSV/Excel reports** |

## Pro Tips

1. **Always compress logs** - 10MB log → 500KB zip (20x reduction!)
2. **Use descriptive zip names** - `app_2025_11_03.zip` not `temp.zip`
3. **Include timestamp in subjects** - `"Daily Report - 2025-11-03"` helps filtering
4. **Monitor email delivery** - Check return value, log failures
5. **Fallback gracefully** - If zip fails, send uncompressed (better than no email)

## Example: Complete Daily Report System

```csharp
public class DailyReportMailer
{
    private readonly string _recipient;
    private readonly string _logDirectory;

    public DailyReportMailer(string recipient, string logDirectory)
    {
        _recipient = recipient;
        _logDirectory = logDirectory;
    }

    public void SendDailyReport(string reportSummary, List<string> errorMessages)
    {
        var today = DateTime.Now.ToString("yyyy-MM-dd");
        var logPath = Path.Combine(_logDirectory, $"app_{DateTime.Now:yyyy_MM_dd}.log");

        // Create error report if there are errors
        string? errorReportPath = null;
        if (errorMessages.Any())
        {
            errorReportPath = Path.Combine(_logDirectory, $"errors_{today}.txt");
            File.WriteAllText(errorReportPath,
                $"Errors on {today}\n" +
                $"Total: {errorMessages.Count}\n\n" +
                string.Join("\n", errorMessages));
        }

        var email = new Email(
            _recipient,
            $"Daily Report - {today}",
            reportSummary
        );

        try
        {
            if (!string.IsNullOrEmpty(errorReportPath))
            {
                email.SendWithZippedAttachmentAndAdditionalFiles(
                    null,
                    logPath,
                    deleteOriginal: false,
                    errorReportPath
                );

                // Clean up error report
                File.Delete(errorReportPath);
            }
            else
            {
                email.SendWithZippedAttachment(null, logPath, deleteOriginal: false);
            }

            Console.WriteLine($"Daily report sent successfully to {_recipient}");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Failed to send daily report: {ex.Message}");
            // Don't throw - logging should continue even if email fails
        }
    }
}
```

## Notes

- **FileShare.ReadWrite is the key trick** for reading locked log files
- **Always clean up zip files** in `finally` blocks
- **Fallback gracefully** - if zip fails, try sending original
- **Compression is worth it** - 10-50x size reduction for logs
- **Test with locked files** - that's the real-world scenario

This pattern has been battle-tested in production for daily automated reports!
