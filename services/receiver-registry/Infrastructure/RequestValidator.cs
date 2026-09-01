using System.Globalization;
using System.Net;
using System.Net.Sockets;
using System.Text.Json;
using QAReceiverRegistry.Models;

namespace QAReceiverRegistry.Infrastructure;

public static class RequestValidator
{
    public static NormalizedReceiver Normalize(CreateReceiverRequest request)
    {
        return NormalizeReceiver(
            request.IpAddress,
            request.Name,
            request.Model,
            request.SerialNumber,
            request.Location,
            request.Notes,
            request.Enabled,
            request.GroupIds,
            request.PrimaryGroupId,
            null);
    }

    public static NormalizedReceiver Normalize(UpdateReceiverRequest request)
    {
        if (request.RowVersion < 1)
        {
            throw new RegistryValidationException("rowVersion must be greater than zero.");
        }

        return NormalizeReceiver(
            request.IpAddress,
            request.Name,
            request.Model,
            request.SerialNumber,
            request.Location,
            request.Notes,
            request.Enabled,
            request.GroupIds,
            request.PrimaryGroupId,
            request.RowVersion);
    }

    public static NormalizedGroup Normalize(CreateGroupRequest request)
    {
        return NormalizeGroup(request.Code, request.Name, request.Description, true, null);
    }

    public static NormalizedGroup Normalize(UpdateGroupRequest request)
    {
        if (request.RowVersion < 1)
        {
            throw new RegistryValidationException("rowVersion must be greater than zero.");
        }

        return NormalizeGroup(request.Code, request.Name, request.Description, request.Enabled, request.RowVersion);
    }

    public static NormalizedFirmwareHistory Normalize(string jobId, UpsertFirmwareHistoryRequest request)
    {
        jobId = NormalizeRequired(jobId, 120, "jobId");
        var ipAddress = request.Ip?.Trim() ?? string.Empty;
        if (!IPAddress.TryParse(ipAddress, out var parsed) || parsed.AddressFamily != AddressFamily.InterNetwork)
        {
            throw new RegistryValidationException("ip must be a valid IPv4 address.");
        }

        var result = NormalizeRequired(request.Result, 40, "result");
        if (result is not ("Completed" or "Failed" or "Cancelled"))
        {
            throw new RegistryValidationException("result must be Completed, Failed, or Cancelled.");
        }

        var errors = (request.Errors ?? [])
            .Select(error => NormalizeRequired(error, 2000, "errors"))
            .Take(50)
            .ToArray();
        if ((request.Errors?.Count ?? 0) > errors.Length)
        {
            throw new RegistryValidationException("errors cannot contain more than 50 entries.");
        }

        var startedAt = NormalizeTimestamp(request.StartedAt, "startedAt");
        DateTimeOffset? completedAt = request.CompletedAt.HasValue && request.CompletedAt.Value.ValueKind != JsonValueKind.Null
            ? NormalizeTimestamp(request.CompletedAt.Value, "completedAt")
            : null;
        if (completedAt.HasValue && completedAt.Value < startedAt)
        {
            throw new RegistryValidationException("completedAt cannot be earlier than startedAt.");
        }

        return new NormalizedFirmwareHistory(
            jobId,
            NormalizeOptional(request.BatchId, 120, "batchId"),
            NormalizeOptional(request.OperationType, 40, "operationType"),
            NormalizeOptional(request.GroupName, 120, "groupName"),
            parsed.ToString(),
            NormalizeOptional(request.ReceiverName, 120, "receiverName"),
            NormalizeRequired(request.Model, 80, "model"),
            NormalizeRequired(request.TargetFile, 500, "targetFile"),
            startedAt,
            completedAt,
            result,
            NormalizeOptional(request.FinalFirmwareVersion, 120, "finalFirmwareVersion"),
            errors);
    }

    private static NormalizedReceiver NormalizeReceiver(
        string ipAddress,
        string? name,
        string? model,
        string? serialNumber,
        string? location,
        string? notes,
        bool enabled,
        IReadOnlyList<long>? groupIds,
        long? primaryGroupId,
        long? rowVersion)
    {
        ipAddress = ipAddress?.Trim() ?? string.Empty;
        if (!IPAddress.TryParse(ipAddress, out var parsed) || parsed.AddressFamily != AddressFamily.InterNetwork)
        {
            throw new RegistryValidationException("ipAddress must be a valid IPv4 address.");
        }

        var normalizedGroups = (groupIds ?? [])
            .Distinct()
            .Order()
            .ToArray();
        if (normalizedGroups.Any(id => id < 1))
        {
            throw new RegistryValidationException("groupIds must contain positive identifiers.");
        }

        if (primaryGroupId.HasValue && !normalizedGroups.Contains(primaryGroupId.Value))
        {
            throw new RegistryValidationException("primaryGroupId must also be present in groupIds.");
        }

        return new NormalizedReceiver(
            parsed.ToString(),
            NormalizeOptional(name, 120, "name"),
            NormalizeOptional(model, 80, "model")?.ToLowerInvariant(),
            NormalizeOptional(serialNumber, 120, "serialNumber"),
            NormalizeOptional(location, 200, "location"),
            NormalizeOptional(notes, 2000, "notes"),
            enabled,
            normalizedGroups,
            primaryGroupId,
            rowVersion);
    }

    private static NormalizedGroup NormalizeGroup(
        string code,
        string name,
        string? description,
        bool enabled,
        long? rowVersion)
    {
        code = code?.Trim().ToUpperInvariant() ?? string.Empty;
        if (code.Length is < 2 or > 40 || code.Any(character => !char.IsAsciiLetterOrDigit(character) && character != '_'))
        {
            throw new RegistryValidationException(
                "code must contain 2-40 uppercase letters, numbers, or underscores.");
        }

        name = name?.Trim() ?? string.Empty;
        if (name.Length is < 2 or > 120)
        {
            throw new RegistryValidationException("name must contain 2-120 characters.");
        }

        return new NormalizedGroup(
            code,
            name,
            NormalizeOptional(description, 500, "description"),
            enabled,
            rowVersion);
    }

    private static string? NormalizeOptional(string? value, int maximumLength, string fieldName)
    {
        value = value?.Trim();
        if (string.IsNullOrEmpty(value))
        {
            return null;
        }

        if (value.Length > maximumLength)
        {
            throw new RegistryValidationException($"{fieldName} cannot exceed {maximumLength} characters.");
        }

        return value;
    }

    private static string NormalizeRequired(string? value, int maximumLength, string fieldName)
    {
        value = value?.Trim() ?? string.Empty;
        if (value.Length is < 1 || value.Length > maximumLength)
        {
            throw new RegistryValidationException($"{fieldName} must contain 1-{maximumLength} characters.");
        }

        return value;
    }

    private static DateTimeOffset NormalizeTimestamp(JsonElement value, string fieldName)
    {
        if (value.ValueKind == JsonValueKind.Number && value.TryGetInt64(out var numericSeconds))
        {
            return FromUnixTimeSeconds(numericSeconds, fieldName);
        }

        if (value.ValueKind == JsonValueKind.String)
        {
            var text = value.GetString()?.Trim() ?? string.Empty;
            if (long.TryParse(text, NumberStyles.Integer, CultureInfo.InvariantCulture, out var stringSeconds))
            {
                return FromUnixTimeSeconds(stringSeconds, fieldName);
            }

            if (DateTimeOffset.TryParse(
                    text,
                    CultureInfo.InvariantCulture,
                    DateTimeStyles.AssumeUniversal | DateTimeStyles.AdjustToUniversal,
                    out var timestamp))
            {
                return timestamp;
            }
        }

        throw new RegistryValidationException($"{fieldName} must be an ISO 8601 timestamp or Unix timestamp in seconds.");
    }

    private static DateTimeOffset FromUnixTimeSeconds(long seconds, string fieldName)
    {
        try
        {
            return DateTimeOffset.FromUnixTimeSeconds(seconds);
        }
        catch (ArgumentOutOfRangeException)
        {
            throw new RegistryValidationException($"{fieldName} contains an invalid Unix timestamp.");
        }
    }
}

public sealed record NormalizedReceiver(
    string IpAddress,
    string? Name,
    string? Model,
    string? SerialNumber,
    string? Location,
    string? Notes,
    bool Enabled,
    IReadOnlyList<long> GroupIds,
    long? PrimaryGroupId,
    long? RowVersion);

public sealed record NormalizedGroup(
    string Code,
    string Name,
    string? Description,
    bool Enabled,
    long? RowVersion);

public sealed record NormalizedFirmwareHistory(
    string JobId,
    string? BatchId,
    string? OperationType,
    string? GroupName,
    string Ip,
    string? ReceiverName,
    string Model,
    string TargetFile,
    DateTimeOffset StartedAt,
    DateTimeOffset? CompletedAt,
    string Result,
    string? FinalFirmwareVersion,
    IReadOnlyList<string> Errors);
