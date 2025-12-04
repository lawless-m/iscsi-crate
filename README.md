# Claude Skills

![Spellbook](spellbook.png)

This folder contains domain-specific skills that teach Claude about proven patterns and implementations used across our projects.

## Available Skills

### 🗄️ Databases
**Description**: RDBMS access patterns for DuckDB, MySQL, PostgreSQL, SQL Server, and DBISAM using ODBC and native drivers

**Key Topics**:
- ServiceLib ODBC wrapper with ExecuteAndMap pattern
- Direct PostgreSQL access with Npgsql (recommended over ODBC)
- Native MySQL access patterns
- DuckDB for querying Parquet files
- PgQuery command-line tool for ad-hoc queries
- Parameter styles by database type

**Reference Files**:
- `ODBC.cs` - ServiceLib ODBC wrapper
- `PgQuery.cs` - PostgreSQL command-line tool

---

### 🔍 Elasticsearch
**Description**: Elasticsearch 5.2 operations using HTTP API - searching, indexing, bulk operations, scroll API, and alias management

**Key Topics**:
- HTTP Client approach (no official client library)
- ServiceLib.Elasticsearch wrapper for common operations
- Scroll API for downloading large indices
- Bulk indexing with NDJSON format
- Zero-downtime updates with timestamped indices + aliases
- Dynamic result parsing

**Reference Files**:
- `Elasticsearch.cs` - ServiceLib HTTP wrapper
- `ElasticsearchService.cs` - JordanPrice service with alias pattern

**Version**: Elasticsearch 5.2 (fixed, will not change)

---

### 📊 Parquet Files
**Description**: Creating and managing Parquet files in C# with multi-threaded operations and incremental updates

**Key Topics**:
- Parquet.Net library (v4.23.5 - 4.25.0)
- Dynamic schema creation from DataTable
- Multi-threaded batch processing patterns
- Thread-safe updates with ParquetUpdateQueue
- Memory management for large datasets
- Incremental sync with timestamp tracking

**Reference Files**:
- `BPQuery_Parquet.cs` - Single-threaded MySQL to Parquet
- `ParquetUpdateQueue.cs` - Thread-safe queue pattern
- `ElastiCompare_ParquetService.cs` - Multi-threaded Elasticsearch to Parquet

**Projects**: BPQuery (MySQL sync), ElastiCompare (Elasticsearch downloads)

---

### 📝 Logging
**Description**: UTF-8 logging extensions and patterns

**Reference Files**:
- `Utf8LoggingExtensions.cs` - UTF-8 logger implementation

---

## How to Use Skills

### Invoke a Skill
Skills are loaded automatically by Claude when relevant to the task, or you can explicitly request them:

**Natural invocation (recommended)**:
- "Help me with Elasticsearch queries" → Claude loads Elasticsearch skill
- "I need to create a Parquet file" → Claude loads Parquet Files skill
- "Set up database connections" → Claude loads Databases skill

**Explicit request**:
- "Use the Databases skill to help me"
- "Load the Parquet Files skill"

Skills are invoked using the Skill tool internally by Claude when needed.

### Skill Structure
Each skill follows this pattern:

```markdown
---
name: Skill Name
description: Brief description
---

# Skill Name

## Instructions
Guidelines for Claude to follow (numbered list)

## Examples
Example scenarios showing usage patterns

---

# Reference Implementation Details
Detailed code examples and patterns
```

### Benefits of Skills
1. **Consistent Patterns**: Ensures Claude uses proven approaches
2. **Self-Contained**: All reference code is in the skills folder
3. **Version Controlled**: Skills folder is the source of truth
4. **Project-Specific**: Based on actual production implementations
5. **Comprehensive**: Combines guidelines, examples, and working code

## Skill Development Tips

### When to Create a Skill
- You have proven patterns used across multiple projects
- You want Claude to follow specific conventions
- You have reusable library code (like ServiceLib)
- You need to document version-specific APIs (like Elasticsearch 5.2)

### Skill File Organization
```
skills/
├── SkillName/
│   ├── SKILL.md              # Main skill documentation
│   ├── Implementation1.cs     # Reference code
│   └── Implementation2.cs     # More reference code
└── README.md                  # This file
```

### Best Practices
1. **Keep Instructions Concise**: 5-10 numbered guidelines
2. **Provide Clear Examples**: Show user request → Claude response pattern
3. **Include Working Code**: Copy actual implementations from projects
4. **Document Constraints**: Note version locks, deprecated patterns
5. **Reference Local Files**: Point to implementation files in the skill folder

## Project References

Skills are based on proven implementations from these projects:

- **BPQuery**: MySQL to Parquet incremental sync
- **ElastiCompare**: Elasticsearch comparison and downloads
- **JordanPrice**: Elasticsearch service with zero-downtime updates
- **CRMPollerFixer**: ODBC database operations
- **PgQuery**: PostgreSQL command-line tool

## Updating Skills

When updating a skill:
1. Update the SKILL.md with new patterns
2. Copy updated implementation files to the skill folder
3. Test by invoking the skill and verifying Claude's responses
4. Document version changes and deprecations

---

*Generated with Claude Code for Matthew Heath's development environment*

Also synced to https://github.com/lawless-m/claude-skills







