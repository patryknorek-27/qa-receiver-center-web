using System.Text.Json;

namespace QAReceiverRegistry.Models;

public sealed record GroupSummary(
    long Id,
    string Code,
    string Name,
    string? Description,
    bool IsSystem,
    bool Enabled,
    long RowVersion,
    bool IsPrimary = false);

public sealed record ReceiverRecord(
    string Id,
    string IpAddress,
    string? Name,
    string? Model,
    string? SerialNumber,
    string? Location,
    string? Notes,
    bool Enabled,
    long RowVersion,
    DateTimeOffset CreatedAt,
    DateTimeOffset UpdatedAt,
    IReadOnlyList<GroupSummary> Groups);

public sealed record CreateReceiverRequest(
    string IpAddress,
    string? Name,
    string? Model,
    string? SerialNumber,
    string? Location,
    string? Notes,
    bool Enabled,
    IReadOnlyList<long>? GroupIds,
    long? PrimaryGroupId);

public sealed record UpdateReceiverRequest(
    string IpAddress,
    string? Name,
    string? Model,
    string? SerialNumber,
    string? Location,
    string? Notes,
    bool Enabled,
    IReadOnlyList<long>? GroupIds,
    long? PrimaryGroupId,
    long RowVersion);

public sealed record CreateGroupRequest(string Code, string Name, string? Description);

public sealed record UpdateGroupRequest(
    string Code,
    string Name,
    string? Description,
    bool Enabled,
    long RowVersion);

public sealed record AuditRecord(
    long Id,
    string EntityType,
    string EntityId,
    string Action,
    string Actor,
    DateTimeOffset ChangedAt,
    string? PayloadJson);

public sealed record SettingRecord(
    string Key,
    JsonElement Value,
    long RowVersion,
    string UpdatedBy,
    DateTimeOffset CreatedAt,
    DateTimeOffset UpdatedAt);

public sealed record UpdateSettingRequest(JsonElement Value, long? RowVersion);

public sealed record FirmwareHistoryRecord(
    string JobId,
    string? BatchId,
    string? OperationType,
    string? GroupName,
    string Ip,
    string? ReceiverName,
    string Model,
    string TargetFile,
    string StartedAt,
    string? CompletedAt,
    string Result,
    string? FinalFirmwareVersion,
    CloneFileSummary? CloneFile,
    IReadOnlyList<string> Errors,
    long RowVersion,
    string CreatedBy,
    string UpdatedBy,
    DateTimeOffset CreatedAt,
    DateTimeOffset UpdatedAt);

public sealed record UpsertFirmwareHistoryRequest(
    string? BatchId,
    string? OperationType,
    string? GroupName,
    string Ip,
    string? ReceiverName,
    string Model,
    string TargetFile,
    JsonElement StartedAt,
    JsonElement? CompletedAt,
    string Result,
    string? FinalFirmwareVersion,
    IReadOnlyList<string>? Errors);

public sealed record CloneFileSummary(
    string Id,
    string FileName,
    long SizeBytes,
    string Sha256,
    string? FirmwareVersion,
    DateTimeOffset CreatedAt);

public sealed record CloneFileDownload(
    string FullPath,
    string FileName,
    long SizeBytes,
    string Sha256);

public sealed record DatabaseStatus(
    string SqliteVersion,
    int SchemaVersion,
    long ReceiverCount,
    string IntegrityStatus);

public sealed record HealthResponse(
    string Status,
    string ServiceVersion,
    DateTimeOffset UtcTime,
    DatabaseStatus Database);

public sealed record BackupResult(string FileName, string FullPath, long SizeBytes, DateTimeOffset CreatedAt);
