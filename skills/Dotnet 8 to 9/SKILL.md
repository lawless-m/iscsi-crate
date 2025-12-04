# .NET 8 to .NET 9 Migration Guide

This skill provides guidance for migrating .NET projects from version 8 to version 9, and for preparing .NET 8 code for easier migration.

## Overview

.NET 9 introduces several improvements and changes that may require adjustments during migration. This guide covers the most common issues encountered when upgrading from .NET 8 to .NET 9.

## Usage Modes

This skill supports two modes of operation:

1. **Pre-Migration Preparation** - Prepare .NET 8 code to be migration-ready (useful when working in environments like Claude for Web that only have .NET 8)
2. **Full Migration** - Complete upgrade from .NET 8 to .NET 9

### Which Mode Should I Use?

**Use Pre-Migration Mode if:**
- You're working in Claude for Web (only has .NET 8 SDK)
- You want to prepare code before migrating
- You want to minimize issues during the actual migration
- You can't update to .NET 9 yet

**Use Full Migration Mode if:**
- You have .NET 9 SDK installed
- You're ready to complete the upgrade
- You've already done pre-migration preparation

---

## Pre-Migration Preparation (for .NET 8 environments)

When you only have access to .NET 8 (e.g., in Claude for Web), you can still prepare your code to make the eventual migration to .NET 9 smoother. This involves fixing compatibility issues proactively.

### Agent Instructions for Pre-Migration

When preparing .NET 8 code for future .NET 9 migration:

1. **Scan for String Literal Issues**
   - Search for verbatim strings containing JavaScript/TypeScript code
   - Look for backtick template literals inside C# `@"..."` strings
   - Replace with string concatenation or raw string literals

2. **Check ImplicitUsings Configuration**
   - Find projects with explicit `Main` methods
   - Ensure `ImplicitUsings` is disabled in those projects to avoid future conflicts

3. **Audit Test Projects**
   - Check if test data files in subdirectories are being compiled
   - Add exclusion rules for TestData folders if needed

4. **Review Package Dependencies**
   - Document packages that will need updates
   - Check for packages like `Microsoft.CodeAnalysis.*` that may need explicit MSBuild workspace references

5. **Document Entry Points**
   - Identify all entry points (Main methods, startup classes)
   - Note any non-standard entry point configurations

### Pre-Migration Checklist

Run this analysis on .NET 8 code before migration:

- [ ] Search for regex pattern: `@"[^"]*\`[^"]*"` (backticks in verbatim strings)
- [ ] Find projects with both `ImplicitUsings` enabled and explicit `Main` methods
- [ ] Identify test projects that include TestData folders without exclusions
- [ ] List all packages using Roslyn/CodeAnalysis APIs
- [ ] Check for any custom MSBuild targets or build scripts
- [ ] Document any warnings in current .NET 8 build

### Pre-Migration Agent Prompt Template

```
I need you to analyze this .NET 8 codebase and prepare it for future migration to .NET 9.

Please perform the following tasks:

1. Search all .cs files for verbatim strings (@"...") containing backticks (`)
   - These will break in .NET 9 due to stricter string literal parsing
   - Fix by converting JavaScript template literals to string concatenation

2. Check all .csproj files for ImplicitUsings + explicit Main method conflicts
   - Projects with Main methods should have <ImplicitUsings>disable</ImplicitUsings>

3. Examine test projects for TestData compilation issues
   - Add <Compile Remove="TestData\**\*.cs" /> if TestData folders exist

4. Review package references for potential .NET 9 incompatibilities
   - Flag any packages below these minimum versions:
     * Microsoft.CodeAnalysis.* < 4.12.0
     * System.Text.Json < 9.0.0
     * Microsoft.Build.Locator < 1.7.8
   - Note: Don't update packages yet (we're still on .NET 8)

5. Create a migration readiness report with findings and fixes applied

Focus on making the code .NET 9-compatible while keeping it on .NET 8.
```

### Automated Pre-Migration Analysis

Use these patterns to detect issues automatically:

#### 1. Detect Problematic String Literals

Use Grep to find verbatim strings with backticks:
```bash
# Search for backticks inside verbatim strings
grep -r '@"[^"]*`' --include="*.cs"
```

**What to look for:**
- Verbatim strings containing JavaScript/TypeScript code
- Template literals with `${...}` syntax
- HTML/CSS embedded in C# strings

**How to fix:**
- Replace backticks with single quotes: `` `text` `` → `'text'`
- Replace template literals: `` `${var}` `` → `' + var + '`
- Ensure HTML attributes use double-double quotes: `class="..."` → `class=""...""`

#### 2. Detect ImplicitUsings Conflicts

Search for projects with potential entry point conflicts:
```bash
# Find Main methods
grep -r "static.*void Main\(" --include="*.cs"

# Check if those projects have ImplicitUsings enabled
grep -r "<ImplicitUsings>enable" --include="*.csproj"
```

**How to fix:**
Add to the `.csproj` with the Main method:
```xml
<ImplicitUsings>disable</ImplicitUsings>
```

#### 3. Detect Test Data Compilation Issues

Look for test projects with TestData directories:
```bash
# Find test projects
find . -name "*Test*.csproj" -o -name "*.Tests.csproj"

# Check for TestData directories
find . -type d -name "TestData"
```

**How to fix:**
Add to test `.csproj`:
```xml
<ItemGroup>
  <Compile Remove="TestData\**\*.cs" />
  <None Include="TestData\**\*" />
</ItemGroup>
```

#### 4. Generate Migration Readiness Report

After scanning, create a report with:
- Count of issues found in each category
- List of files requiring changes
- Suggested fixes for each issue
- Estimated migration complexity (Low/Medium/High)
- Package versions that will need updates

---

## Full Migration Process (.NET 8 → .NET 9)

### 1. Update Target Framework

Update all `.csproj` files to target .NET 9:

```xml
<PropertyGroup>
  <TargetFramework>net9.0</TargetFramework>
</PropertyGroup>
```

### 2. Update NuGet Packages

Update package references to versions compatible with .NET 9:

**Common Package Updates:**
- `Microsoft.CodeAnalysis.CSharp.Workspaces`: 4.12.0+
- `Microsoft.CodeAnalysis.Workspaces.MSBuild`: 4.12.0+
- `Microsoft.Build.Locator`: 1.7.8+
- `System.Text.Json`: 9.0.0+
- `Microsoft.NET.Test.Sdk`: 17.11.1+
- `xunit`: 2.9.2+
- `xunit.runner.visualstudio`: 2.8.2+

### 3. Common Breaking Changes and Fixes

#### String Literal Parsing Changes

.NET 9 has stricter parsing of string literals, especially in verbatim strings (`@"..."`).

**Issue:** JavaScript template literals (backticks) in C# verbatim strings cause errors.

**Before (breaks in .NET 9):**
```csharp
return @"
html += `<div class=""result"">${value}</div>`;
";
```

**After (works in .NET 9):**
```csharp
return @"
html += '<div class=""result"">' + value + '</div>';
";
```

**Fix:** Replace JavaScript template literals with string concatenation inside C# verbatim strings, and use double quotes (`""`) for HTML attributes.

#### ImplicitUsings Behavior

.NET 9 generates more implicit usings and may create conflicts with explicit entry points.

**Issue:** Multiple entry points when `ImplicitUsings` is enabled in projects with explicit `Main` methods.

**Fix:** Set `<ImplicitUsings>disable</ImplicitUsings>` for projects that define their own `Main` method:

```xml
<PropertyGroup>
  <ImplicitUsings>disable</ImplicitUsings>
</PropertyGroup>
```

#### Test Data Files

Test projects may accidentally compile test data files.

**Issue:** Test data `.cs` files being compiled as part of test project, causing compilation errors.

**Fix:** Explicitly exclude test data from compilation in the test `.csproj`:

```xml
<ItemGroup>
  <Compile Remove="TestData\**\*.cs" />
  <None Include="TestData\**\*" />
</ItemGroup>
```

### 4. Missing Package References

Some packages that were implicitly referenced in .NET 8 may need explicit references in .NET 9.

**Example:** `Microsoft.CodeAnalysis.Workspaces.MSBuild` often needs to be explicitly added:

```xml
<PackageReference Include="Microsoft.CodeAnalysis.Workspaces.MSBuild" Version="4.12.0" />
```

### 5. Build and Test

After making changes:

1. Clean and restore packages:
   ```bash
   dotnet clean
   dotnet restore
   ```

2. Build the solution:
   ```bash
   dotnet build
   ```

3. Run all tests:
   ```bash
   dotnet test
   ```

### 6. Verify Migration Success

- [ ] All projects build without errors
- [ ] All tests pass
- [ ] No new warnings introduced
- [ ] Application runs as expected
- [ ] Performance is comparable or better

## Migration Checklists

### Pre-Migration Preparation Checklist (in .NET 8)

Use this before you have access to .NET 9:

- [ ] Scan for and fix string literal issues (backticks in verbatim strings)
- [ ] Configure `ImplicitUsings` correctly for projects with Main methods
- [ ] Exclude test data files from test project compilation
- [ ] Document packages that need version updates
- [ ] Run full test suite and document baseline
- [ ] Create migration readiness report
- [ ] Commit all pre-migration fixes

### Full Migration Checklist (migrating to .NET 9)

Use this when performing the actual migration:

- [ ] Complete pre-migration preparation (above)
- [ ] Update all `.csproj` files to target `net9.0`
- [ ] Update all NuGet packages to .NET 9 compatible versions
- [ ] Add any missing explicit package references (e.g., MSBuild workspaces)
- [ ] Verify build succeeds with no errors
- [ ] Run all tests to ensure functionality is preserved
- [ ] Review and address any new warnings
- [ ] Test application thoroughly
- [ ] Update CI/CD pipelines for .NET 9
- [ ] Update documentation and README

## Common Errors and Solutions

### Error: CS1002, CS1056, CS1010 - String Literal Errors

**Cause:** JavaScript template literals in C# verbatim strings.

**Solution:** Replace backtick template literals with string concatenation.

### Error: CS0017 - Multiple Entry Points

**Cause:** `ImplicitUsings` generating a Program class when one already exists.

**Solution:** Set `<ImplicitUsings>disable</ImplicitUsings>` in the project file.

### Error: CS0234 - Namespace Not Found

**Cause:** Missing package reference (e.g., `Microsoft.CodeAnalysis.MSBuild`).

**Solution:** Add the missing package reference to the `.csproj` file.

### Error: Test Data Files Causing Compilation Errors

**Cause:** Test data `.cs` files being included in test project compilation.

**Solution:** Exclude test data from compilation using `<Compile Remove="TestData\**\*.cs" />`.

## Real-World Example: Pre-Migration Fix

### Before (breaks in .NET 9):

```csharp
// In HtmlOutput.cs
private string GenerateJavaScript()
{
    return @"
document.addEventListener('DOMContentLoaded', function() {
    const searchBox = document.getElementById('searchBox');

    searchBox.addEventListener('input', function(e) {
        const query = e.target.value.toLowerCase();
        let html = '<div class=""results"">';
        matches.forEach(match => {
            html += `<div class=""result""><a href=""${match.href}"">${match.text}</a></div>`;
        });
        html += '</div>';
        results.innerHTML = html;
    });
});
";
}
```

**Problem:** Backtick template literals (`` `...` ``) inside C# verbatim string will cause CS1002, CS1056, CS1010 errors in .NET 9.

### After (works in both .NET 8 and 9):

```csharp
// In HtmlOutput.cs - Fixed version
private string GenerateJavaScript()
{
    return @"
document.addEventListener('DOMContentLoaded', function() {
    const searchBox = document.getElementById('searchBox');

    searchBox.addEventListener('input', function(e) {
        const query = e.target.value.toLowerCase();
        let html = '<div class=""results"">';
        matches.forEach(match => {
            html += '<div class=""result""><a href=""' + match.href + '"">' + match.text + '</a></div>';
        });
        html += '</div>';
        results.innerHTML = html;
    });
});
";
}
```

**Changes made:**
- Replaced backticks with single quotes
- Replaced `${match.href}` with `' + match.href + '`
- Replaced `${match.text}` with `' + match.text + '`
- Kept `""` for HTML attributes inside the verbatim string

This code now works in both .NET 8 and .NET 9!

## Quick Reference Commands

### For Pre-Migration Analysis (in .NET 8):

```bash
# Find string literal issues
grep -r "@\"" --include="*.cs" | grep "\`"

# Find Main methods
grep -r "static.*void Main\(" --include="*.cs"

# Find test projects
find . -name "*.Tests.csproj" -o -name "*Test*.csproj"

# Check current build status
dotnet build --no-restore
```

### For Full Migration (when you have .NET 9):

```bash
# Update to .NET 9
find . -name "*.csproj" -exec sed -i 's/<TargetFramework>net8.0<\/TargetFramework>/<TargetFramework>net9.0<\/TargetFramework>/g' {} \;

# Clean and restore
dotnet clean
dotnet restore

# Build and test
dotnet build
dotnet test
```

## Additional Resources

- [.NET 9 Release Notes](https://learn.microsoft.com/en-us/dotnet/core/whats-new/dotnet-9/overview)
- [Breaking Changes in .NET 9](https://learn.microsoft.com/en-us/dotnet/core/compatibility/9.0)
- [.NET 9 Migration Guide](https://learn.microsoft.com/en-us/dotnet/core/porting/)
