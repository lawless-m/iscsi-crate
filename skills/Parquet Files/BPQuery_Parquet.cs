using Parquet;
using Parquet.Data;
using Parquet.Schema;
using Microsoft.Data.Analysis;
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Data;
using ParquetDataColumn = Parquet.Data.DataColumn;
using SystemDataColumn = System.Data.DataColumn;
using System.Threading.Tasks;

namespace BPQuery;

public class Parquet
{
    private static Array ConvertToTypedArray(List<object> values, Type targetType)
    {
        if (targetType == typeof(string))
        {
            return values.Select(v => v?.ToString() ?? "").ToArray();
        }
        else if (targetType == typeof(int))
        {
            return values.Select(v => v == null || v.ToString() == "" ? 0 : Convert.ToInt32(v)).ToArray();
        }
        else if (targetType == typeof(long))
        {
            return values.Select(v => v == null || v.ToString() == "" ? 0L : Convert.ToInt64(v)).ToArray();
        }
        else if (targetType == typeof(double))
        {
            return values.Select(v => v == null || v.ToString() == "" ? 0.0 : Convert.ToDouble(v)).ToArray();
        }
        else if (targetType == typeof(bool))
        {
            return values.Select(v => v == null || v.ToString() == "" ? false : Convert.ToBoolean(v)).ToArray();
        }
        else if (targetType == typeof(DateTime))
        {
            return values.Select(v => v == null || v.ToString() == "" ? DateTime.MinValue : Convert.ToDateTime(v)).ToArray();
        }
        else
        {
            return values.Select(v => v?.ToString() ?? "").ToArray();
        }
    }
    public static async Task SaveDTAsync(DataTable dt, string filePath)
    {
        var fields = new List<Field>();

        foreach (SystemDataColumn col in dt.Columns)
        {
            switch (col.DataType.Name)
            {
                case "String":
                    fields.Add(new DataField<string>(col.ColumnName));
                    break;
                case "Int32":
                    fields.Add(new DataField<int>(col.ColumnName));
                    break;
                case "Int64":
                    fields.Add(new DataField<long>(col.ColumnName));
                    break;
                case "Double":
                    fields.Add(new DataField<double>(col.ColumnName));
                    break;
                case "Decimal":
                    fields.Add(new DataField<double>(col.ColumnName));
                    break;
                case "Boolean":
                    fields.Add(new DataField<bool>(col.ColumnName));
                    break;
                case "DateTime":
                    fields.Add(new DataField<DateTime>(col.ColumnName));
                    break;
                default:
                    fields.Add(new DataField<string>(col.ColumnName));
                    break;
            }
        }

        var schema = new ParquetSchema(fields);
        using var stream = new FileStream(filePath, FileMode.Create, FileAccess.Write);
        using var writer = await ParquetWriter.CreateAsync(schema, stream);
        using var rg = writer.CreateRowGroup();

        for (int colIndex = 0; colIndex < dt.Columns.Count; colIndex++)
        {
            var col = dt.Columns[colIndex];
            var values = new List<object>();

            foreach (DataRow row in dt.Rows)
            {
                var val = row[col];
                if (val == DBNull.Value)
                {
                    values.Add("");
                }
                else if (col.DataType == typeof(decimal))
                {
                    values.Add(Convert.ToDouble(val));
                }
                else
                {
                    values.Add(val);
                }
            }

            await rg.WriteColumnAsync(new ParquetDataColumn((DataField)fields[colIndex], ConvertToTypedArray(values, ((DataField)fields[colIndex]).ClrType)));
        }
    }

    public static async Task SaveMultipleDTAsync(List<DataTable> dataTables, string filePath)
    {
        if (dataTables == null || dataTables.Count == 0)
            return;

        var firstTable = dataTables[0];
        var fields = new List<Field>();

        foreach (SystemDataColumn col in firstTable.Columns)
        {
            switch (col.DataType.Name)
            {
                case "String":
                    fields.Add(new DataField<string>(col.ColumnName));
                    break;
                case "Int32":
                    fields.Add(new DataField<int>(col.ColumnName));
                    break;
                case "Int64":
                    fields.Add(new DataField<long>(col.ColumnName));
                    break;
                case "Double":
                    fields.Add(new DataField<double>(col.ColumnName));
                    break;
                case "Decimal":
                    fields.Add(new DataField<double>(col.ColumnName));
                    break;
                case "Boolean":
                    fields.Add(new DataField<bool>(col.ColumnName));
                    break;
                case "DateTime":
                    fields.Add(new DataField<DateTime>(col.ColumnName));
                    break;
                default:
                    fields.Add(new DataField<string>(col.ColumnName));
                    break;
            }
        }

        var schema = new ParquetSchema(fields);
        using var stream = new FileStream(filePath, FileMode.Create, FileAccess.Write);
        using var writer = await ParquetWriter.CreateAsync(schema, stream);

        foreach (var dataTable in dataTables)
        {
            using var rg = writer.CreateRowGroup();

            for (int colIdx = 0; colIdx < dataTable.Columns.Count; colIdx++)
            {
                var col = dataTable.Columns[colIdx];
                var values = new List<object>();

                foreach (DataRow row in dataTable.Rows)
                {
                    var val = row[col];
                    if (val == DBNull.Value)
                    {
                        values.Add("");
                    }
                    else if (col.DataType == typeof(decimal))
                    {
                        values.Add(Convert.ToDouble(val));
                    }
                    else
                    {
                        values.Add(val);
                    }
                }

                await rg.WriteColumnAsync(new ParquetDataColumn((DataField)fields[colIdx], ConvertToTypedArray(values, ((DataField)fields[colIdx]).ClrType)));
            }
        }
    }

    public static async Task AppendMultipleDTAsync(List<DataTable> dataTables, string filePath)
    {
        if (dataTables == null || dataTables.Count == 0)
            return;

        // If file doesn't exist, create it normally
        if (!File.Exists(filePath))
        {
            await SaveMultipleDTAsync(dataTables, filePath);
            return;
        }

        var firstTable = dataTables[0];
        var fields = new List<Field>();

        foreach (SystemDataColumn col in firstTable.Columns)
        {
            switch (col.DataType.Name)
            {
                case "String":
                    fields.Add(new DataField<string>(col.ColumnName));
                    break;
                case "Int32":
                    fields.Add(new DataField<int>(col.ColumnName));
                    break;
                case "Int64":
                    fields.Add(new DataField<long>(col.ColumnName));
                    break;
                case "Double":
                    fields.Add(new DataField<double>(col.ColumnName));
                    break;
                case "Decimal":
                    fields.Add(new DataField<double>(col.ColumnName));
                    break;
                case "Boolean":
                    fields.Add(new DataField<bool>(col.ColumnName));
                    break;
                case "DateTime":
                    fields.Add(new DataField<DateTime>(col.ColumnName));
                    break;
                default:
                    fields.Add(new DataField<string>(col.ColumnName));
                    break;
            }
        }

        var schema = new ParquetSchema(fields);

        // For Parquet.Net, true appending requires opening the file in append mode
        using var stream = new FileStream(filePath, FileMode.Open, FileAccess.ReadWrite);
        stream.Seek(0, SeekOrigin.End); // Move to end for appending

        using var writer = await ParquetWriter.CreateAsync(schema, stream, append: true);

        foreach (var dataTable in dataTables)
        {
            using var rg = writer.CreateRowGroup();

            for (int colIdx = 0; colIdx < dataTable.Columns.Count; colIdx++)
            {
                var col = dataTable.Columns[colIdx];
                var values = new List<object>();

                foreach (DataRow row in dataTable.Rows)
                {
                    var val = row[col];
                    if (val == DBNull.Value)
                    {
                        values.Add("");
                    }
                    else if (col.DataType == typeof(decimal))
                    {
                        values.Add(Convert.ToDouble(val));
                    }
                    else
                    {
                        values.Add(val);
                    }
                }

                await rg.WriteColumnAsync(new ParquetDataColumn((DataField)fields[colIdx], ConvertToTypedArray(values, ((DataField)fields[colIdx]).ClrType)));
            }
        }
    }

    public static async Task WriteParquetAsync(string path, Dictionary<string, List<object>> data)
    {
        // Validate all lists have the same length
        if (data.Count == 0)
            throw new ArgumentException("Dictionary cannot be empty");

        var firstLength = data.First().Value.Count;
        if (!data.Values.All(list => list.Count == firstLength))
            throw new ArgumentException("All lists must have the same length");

        // Create column definitions based on data types
        var fields = new List<Field>();
        foreach (var kvp in data)
        {
            var columnName = kvp.Key;
            var columnData = kvp.Value;

            // Determine column type from first non-null value
            var firstValue = columnData.FirstOrDefault(x => x != null);
            if (firstValue == null)
                throw new ArgumentException($"Column {columnName} contains only null values");

            switch (firstValue)
            {
                case int:
                    fields.Add(new DataField<int>(columnName));
                    break;
                case long:
                    fields.Add(new DataField<long>(columnName));
                    break;
                case float:
                    fields.Add(new DataField<float>(columnName));
                    break;
                case double:
                    fields.Add(new DataField<double>(columnName));
                    break;
                case string:
                    fields.Add(new DataField<string>(columnName));
                    break;
                case bool:
                    fields.Add(new DataField<bool>(columnName));
                    break;
                default:
                    throw new ArgumentException($"Unsupported data type for column {columnName}: {firstValue.GetType()}");
            }
        }

        var schema = new ParquetSchema(fields);
        using var stream = new FileStream(path, FileMode.Create, FileAccess.Write);
        using var writer = await ParquetWriter.CreateAsync(schema, stream);
        using var rg = writer.CreateRowGroup();

        // Write each column's data
        foreach (var kvp in data)
        {
            var columnName = kvp.Key;
            var columnData = kvp.Value;
            var field = fields.First(f => f.Name == columnName);

            await rg.WriteColumnAsync(new ParquetDataColumn((DataField)field, ConvertToTypedArray(columnData, ((DataField)field).ClrType)));
        }
    }
}

public class ParquetTests
{
    private static async Task<bool> CreateParquet()
    {
        try
        {
            var data = new Dictionary<string, List<object>>
            {
                { "Text", new List<object> { "Hello", "World", "Test", "Data" } },
                { "Nums", new List<object> { 1, 2, 3, 4 } }
            };
            await Parquet.WriteParquetAsync("test.parquet", data);
            if (File.Exists("test.parquet"))
            {
                File.Delete("test.parquet");
                return true;
            }
        }
        catch (Exception e)
        {
            Console.WriteLine("FAILED: Parquet file not created");
            Console.WriteLine(e.Message);
            return false;
        }
        return false;
    }
}