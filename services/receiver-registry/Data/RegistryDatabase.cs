using System.Globalization;
using System.Security.Cryptography;
using System.Text.Json;
using Microsoft.Data.Sqlite;
using Microsoft.Extensions.Options;
using QAReceiverRegistry.Configuration;
using QAReceiverRegistry.Infrastructure;
using QAReceiverRegistry.Models;

namespace QAReceiverRegistry.Data;

public sealed class RegistryDatabase
{
    private const long MaximumCloneFileSizeBytes = 20L * 1024 * 1024;
    private readonly RegistryOptions _options;
    private readonly ILogger<RegistryDatabase> _logger;
    private readonly string _connectionString;

    public RegistryDatabase(IOptions<RegistryOptions> options, ILogger<RegistryDatabase> logger)
    {
        _options = options.Value;
        _logger = logger;
        _connectionString = new SqliteConnectionStringBuilder
        {
            DataSource = _options.DatabasePath,
            Mode = SqliteOpenMode.ReadWriteCreate,
            Cache = SqliteCacheMode.Shared,
            Pooling = true,
            ForeignKeys = true,
            DefaultTimeout = _options.BusyTimeoutSeconds,
        }.ToString();
    }

    public async Task InitializeAsync(CancellationToken cancellationToken = default)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(_options.DatabasePath)!);
        Directory.CreateDirectory(_options.CloneFilesPath);
        Directory.CreateDirectory(_options.BackupPath);

        await using var connection = await OpenConnectionAsync(cancellationToken);
        await ExecuteNonQueryAsync(connection, null, "PRAGMA journal_mode=WAL;", cancellationToken);
        await ExecuteNonQueryAsync(connection, null, "PRAGMA synchronous=NORMAL;", cancellationToken);
        await ExecuteNonQueryAsync(connection, null, "PRAGMA foreign_keys=ON;", cancellationToken);
        await ExecuteNonQueryAsync(
            connection,
            null,
            $"PRAGMA busy_timeout={_options.BusyTimeoutSeconds * 1000};",
            cancellationToken);
        await ExecuteNonQueryAsync(
            connection,
            null,
            """
            CREATE TABLE IF NOT EXISTS _schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TEXT NOT NULL
            );
            """,
            cancellationToken);

        var migrationDirectory = Path.Combine(AppContext.BaseDirectory, "Migrations");
        if (!Directory.Exists(migrationDirectory))
        {
            throw new InvalidOperationException($"Migration directory was not found: {migrationDirectory}");
        }

        foreach (var migrationPath in Directory.GetFiles(migrationDirectory, "*.sql").OrderBy(Path.GetFileName))
        {
            var fileName = Path.GetFileName(migrationPath);
            var versionText = fileName.Split('_', 2)[0];
            if (!int.TryParse(versionText, NumberStyles.None, CultureInfo.InvariantCulture, out var version))
            {
                throw new InvalidOperationException($"Migration file has an invalid name: {fileName}");
            }

            if (await IsMigrationAppliedAsync(connection, version, cancellationToken))
            {
                continue;
            }

            await using var transaction = (SqliteTransaction)await connection.BeginTransactionAsync(cancellationToken);
            var sql = await File.ReadAllTextAsync(migrationPath, cancellationToken);
            await ExecuteNonQueryAsync(connection, transaction, sql, cancellationToken);
            await ExecuteNonQueryAsync(
                connection,
                transaction,
                "INSERT INTO _schema_migrations(version, name, applied_at) VALUES ($version, $name, $appliedAt);",
                cancellationToken,
                ("$version", version),
                ("$name", fileName),
                ("$appliedAt", UtcNowText()));
            await transaction.CommitAsync(cancellationToken);
            _logger.LogInformation("Applied database migration {Migration}", fileName);
        }
    }

    public async Task<IReadOnlyList<ReceiverRecord>> ListReceiversAsync(
        bool includeDisabled,
        CancellationToken cancellationToken = default)
    {
        await using var connection = await OpenConnectionAsync(cancellationToken);
        var receivers = new List<ReceiverRecord>();
        await using var command = connection.CreateCommand();
        command.CommandText =
            """
            SELECT id, ip_address, name, model, serial_number, location, notes, enabled,
                   row_version, created_at, updated_at
            FROM receivers
            WHERE $includeDisabled = 1 OR enabled = 1
            ORDER BY ip_address COLLATE NOCASE;
            """;
        command.Parameters.AddWithValue("$includeDisabled", includeDisabled ? 1 : 0);

        await using (var reader = await command.ExecuteReaderAsync(cancellationToken))
        {
            while (await reader.ReadAsync(cancellationToken))
            {
                receivers.Add(ReadReceiver(reader, []));
            }
        }

        var groups = await LoadMembershipsAsync(connection, receivers.Select(receiver => receiver.Id), cancellationToken);
        return receivers
            .OrderBy(receiver => System.Net.IPAddress.Parse(receiver.IpAddress).GetAddressBytes(), ByteArrayComparer.Instance)
            .Select(receiver => receiver with
            {
                Groups = groups.TryGetValue(receiver.Id, out var membership) ? membership : [],
            })
            .ToArray();
    }

    public async Task<ReceiverRecord> GetReceiverAsync(string id, CancellationToken cancellationToken = default)
    {
        await using var connection = await OpenConnectionAsync(cancellationToken);
        var receiver = await FindReceiverAsync(connection, null, id, cancellationToken)
            ?? throw new EntityNotFoundException($"Receiver {id} was not found.");
        var groups = await LoadMembershipsAsync(connection, [id], cancellationToken);
        return receiver with { Groups = groups.TryGetValue(id, out var membership) ? membership : [] };
    }

    public async Task<ReceiverRecord> CreateReceiverAsync(
        CreateReceiverRequest request,
        string actor,
        CancellationToken cancellationToken = default)
    {
        var value = RequestValidator.Normalize(request);
        var id = Guid.NewGuid().ToString("D");
        var now = UtcNowText();
        await using var connection = await OpenConnectionAsync(cancellationToken);
        await using var transaction = (SqliteTransaction)await connection.BeginTransactionAsync(cancellationToken);
        await ValidateGroupsAsync(connection, transaction, value.GroupIds, cancellationToken);

        try
        {
            await ExecuteNonQueryAsync(
                connection,
                transaction,
                """
                INSERT INTO receivers
                    (id, ip_address, name, model, serial_number, location, notes, enabled,
                     row_version, created_at, updated_at)
                VALUES
                    ($id, $ip, $name, $model, $serial, $location, $notes, $enabled, 1, $now, $now);
                """,
                cancellationToken,
                ("$id", id), ("$ip", value.IpAddress), ("$name", value.Name),
                ("$model", value.Model), ("$serial", value.SerialNumber),
                ("$location", value.Location), ("$notes", value.Notes),
                ("$enabled", value.Enabled ? 1 : 0), ("$now", now));
        }
        catch (SqliteException exception) when (exception.SqliteErrorCode == 19)
        {
            throw new DuplicateEntityException($"A receiver with IP address {value.IpAddress} already exists.");
        }

        await ReplaceMembershipsAsync(connection, transaction, id, value.GroupIds, value.PrimaryGroupId, now, cancellationToken);
        await WriteAuditAsync(connection, transaction, "receiver", id, "created", actor, value, cancellationToken);
        await transaction.CommitAsync(cancellationToken);
        return await GetReceiverAsync(id, cancellationToken);
    }

    public async Task<ReceiverRecord> UpdateReceiverAsync(
        string id,
        UpdateReceiverRequest request,
        string actor,
        CancellationToken cancellationToken = default)
    {
        var value = RequestValidator.Normalize(request);
        var now = UtcNowText();
        await using var connection = await OpenConnectionAsync(cancellationToken);
        await using var transaction = (SqliteTransaction)await connection.BeginTransactionAsync(cancellationToken);
        await ValidateGroupsAsync(connection, transaction, value.GroupIds, cancellationToken);

        int changed;
        try
        {
            changed = await ExecuteNonQueryAsync(
                connection,
                transaction,
                """
                UPDATE receivers
                SET ip_address = $ip,
                    name = $name,
                    model = $model,
                    serial_number = $serial,
                    location = $location,
                    notes = $notes,
                    enabled = $enabled,
                    row_version = row_version + 1,
                    updated_at = $now
                WHERE id = $id AND row_version = $rowVersion;
                """,
                cancellationToken,
                ("$id", id), ("$ip", value.IpAddress), ("$name", value.Name),
                ("$model", value.Model), ("$serial", value.SerialNumber),
                ("$location", value.Location), ("$notes", value.Notes),
                ("$enabled", value.Enabled ? 1 : 0), ("$now", now),
                ("$rowVersion", value.RowVersion));
        }
        catch (SqliteException exception) when (exception.SqliteErrorCode == 19)
        {
            throw new DuplicateEntityException($"A receiver with IP address {value.IpAddress} already exists.");
        }

        if (changed == 0)
        {
            await ThrowNotFoundOrConflictAsync(connection, transaction, "receivers", id, cancellationToken);
        }

        await ReplaceMembershipsAsync(connection, transaction, id, value.GroupIds, value.PrimaryGroupId, now, cancellationToken);
        await WriteAuditAsync(connection, transaction, "receiver", id, "updated", actor, value, cancellationToken);
        await transaction.CommitAsync(cancellationToken);
        return await GetReceiverAsync(id, cancellationToken);
    }

    public async Task DisableReceiverAsync(
        string id,
        long rowVersion,
        string actor,
        CancellationToken cancellationToken = default)
    {
        if (rowVersion < 1)
        {
            throw new RegistryValidationException("rowVersion must be greater than zero.");
        }

        await using var connection = await OpenConnectionAsync(cancellationToken);
        await using var transaction = (SqliteTransaction)await connection.BeginTransactionAsync(cancellationToken);
        var changed = await ExecuteNonQueryAsync(
            connection,
            transaction,
            """
            UPDATE receivers
            SET enabled = 0, row_version = row_version + 1, updated_at = $now
            WHERE id = $id AND row_version = $rowVersion;
            """,
            cancellationToken,
            ("$id", id), ("$rowVersion", rowVersion), ("$now", UtcNowText()));
        if (changed == 0)
        {
            await ThrowNotFoundOrConflictAsync(connection, transaction, "receivers", id, cancellationToken);
        }

        await WriteAuditAsync(connection, transaction, "receiver", id, "disabled", actor, new { rowVersion }, cancellationToken);
        await transaction.CommitAsync(cancellationToken);
    }

    public async Task<IReadOnlyList<GroupSummary>> ListGroupsAsync(
        bool includeDisabled,
        CancellationToken cancellationToken = default)
    {
        await using var connection = await OpenConnectionAsync(cancellationToken);
        await using var command = connection.CreateCommand();
        command.CommandText =
            """
            SELECT id, code, name, description, is_system, enabled, row_version
            FROM receiver_groups
            WHERE $includeDisabled = 1 OR enabled = 1
            ORDER BY id;
            """;
        command.Parameters.AddWithValue("$includeDisabled", includeDisabled ? 1 : 0);
        var groups = new List<GroupSummary>();
        await using var reader = await command.ExecuteReaderAsync(cancellationToken);
        while (await reader.ReadAsync(cancellationToken))
        {
            groups.Add(ReadGroup(reader, false));
        }

        return groups;
    }

    public async Task<GroupSummary> CreateGroupAsync(
        CreateGroupRequest request,
        string actor,
        CancellationToken cancellationToken = default)
    {
        var value = RequestValidator.Normalize(request);
        var now = UtcNowText();
        await using var connection = await OpenConnectionAsync(cancellationToken);
        await using var transaction = (SqliteTransaction)await connection.BeginTransactionAsync(cancellationToken);
        try
        {
            await ExecuteNonQueryAsync(
                connection,
                transaction,
                """
                INSERT INTO receiver_groups
                    (code, name, description, is_system, enabled, row_version, created_at, updated_at)
                VALUES ($code, $name, $description, 0, 1, 1, $now, $now);
                """,
                cancellationToken,
                ("$code", value.Code), ("$name", value.Name),
                ("$description", value.Description), ("$now", now));
        }
        catch (SqliteException exception) when (exception.SqliteErrorCode == 19)
        {
            throw new DuplicateEntityException($"A group with code {value.Code} or name {value.Name} already exists.");
        }

        await using var idCommand = connection.CreateCommand();
        idCommand.Transaction = transaction;
        idCommand.CommandText = "SELECT last_insert_rowid();";
        var id = Convert.ToInt64(await idCommand.ExecuteScalarAsync(cancellationToken), CultureInfo.InvariantCulture);
        await WriteAuditAsync(connection, transaction, "group", id.ToString(CultureInfo.InvariantCulture), "created", actor, value, cancellationToken);
        await transaction.CommitAsync(cancellationToken);
        return (await ListGroupsAsync(true, cancellationToken)).Single(group => group.Id == id);
    }

    public async Task<GroupSummary> UpdateGroupAsync(
        long id,
        UpdateGroupRequest request,
        string actor,
        CancellationToken cancellationToken = default)
    {
        var value = RequestValidator.Normalize(request);
        await using var connection = await OpenConnectionAsync(cancellationToken);
        await using var transaction = (SqliteTransaction)await connection.BeginTransactionAsync(cancellationToken);
        await ValidateSystemGroupUpdateAsync(connection, transaction, id, value, cancellationToken);
        int changed;
        try
        {
            changed = await ExecuteNonQueryAsync(
                connection,
                transaction,
                """
                UPDATE receiver_groups
                SET code = $code,
                    name = $name,
                    description = $description,
                    enabled = $enabled,
                    row_version = row_version + 1,
                    updated_at = $now
                WHERE id = $id AND row_version = $rowVersion;
                """,
                cancellationToken,
                ("$id", id), ("$code", value.Code), ("$name", value.Name),
                ("$description", value.Description), ("$enabled", value.Enabled ? 1 : 0),
                ("$rowVersion", value.RowVersion), ("$now", UtcNowText()));
        }
        catch (SqliteException exception) when (exception.SqliteErrorCode == 19)
        {
            throw new DuplicateEntityException($"A group with code {value.Code} or name {value.Name} already exists.");
        }

        if (changed == 0)
        {
            await ThrowNotFoundOrConflictAsync(connection, transaction, "receiver_groups", id.ToString(CultureInfo.InvariantCulture), cancellationToken);
        }

        await WriteAuditAsync(connection, transaction, "group", id.ToString(CultureInfo.InvariantCulture), "updated", actor, value, cancellationToken);
        await transaction.CommitAsync(cancellationToken);
        return (await ListGroupsAsync(true, cancellationToken)).Single(group => group.Id == id);
    }

    public async Task<IReadOnlyList<AuditRecord>> ListAuditAsync(int limit, CancellationToken cancellationToken = default)
    {
        limit = Math.Clamp(limit, 1, 500);
        await using var connection = await OpenConnectionAsync(cancellationToken);
        await using var command = connection.CreateCommand();
        command.CommandText =
            """
            SELECT id, entity_type, entity_id, action, actor, changed_at, payload_json
            FROM audit_log
            ORDER BY id DESC
            LIMIT $limit;
            """;
        command.Parameters.AddWithValue("$limit", limit);
        var records = new List<AuditRecord>();
        await using var reader = await command.ExecuteReaderAsync(cancellationToken);
        while (await reader.ReadAsync(cancellationToken))
        {
            records.Add(new AuditRecord(
                reader.GetInt64(0), reader.GetString(1), reader.GetString(2), reader.GetString(3),
                reader.GetString(4), ParseTimestamp(reader.GetString(5)), GetNullableString(reader, 6)));
        }

        return records;
    }

    public async Task<IReadOnlyList<SettingRecord>> ListSettingsAsync(
        CancellationToken cancellationToken = default)
    {
        await using var connection = await OpenConnectionAsync(cancellationToken);
        await using var command = connection.CreateCommand();
        command.CommandText =
            """
            SELECT setting_key, value_json, row_version, updated_by, created_at, updated_at
            FROM app_settings
            ORDER BY setting_key COLLATE NOCASE;
            """;
        var settings = new List<SettingRecord>();
        await using var reader = await command.ExecuteReaderAsync(cancellationToken);
        while (await reader.ReadAsync(cancellationToken))
        {
            settings.Add(ReadSetting(reader));
        }

        return settings;
    }

    public async Task<SettingRecord> GetSettingAsync(
        string key,
        CancellationToken cancellationToken = default)
    {
        key = NormalizeSettingKey(key);
        await using var connection = await OpenConnectionAsync(cancellationToken);
        return await FindSettingAsync(connection, null, key, cancellationToken)
            ?? throw new EntityNotFoundException($"Setting {key} was not found.");
    }

    public async Task<SettingRecord> SaveSettingAsync(
        string key,
        UpdateSettingRequest request,
        string actor,
        CancellationToken cancellationToken = default)
    {
        key = NormalizeSettingKey(key);
        var valueJson = NormalizeSettingValue(request.Value);
        actor = NormalizeActor(actor);
        var now = UtcNowText();
        await using var connection = await OpenConnectionAsync(cancellationToken);
        await using var transaction = (SqliteTransaction)await connection.BeginTransactionAsync(cancellationToken);
        var current = await FindSettingAsync(connection, transaction, key, cancellationToken);

        if (current is null)
        {
            if (request.RowVersion.HasValue)
            {
                throw new ConcurrencyConflictException("The setting no longer exists in the expected state. Reload it and try again.");
            }

            await ExecuteNonQueryAsync(
                connection,
                transaction,
                """
                INSERT INTO app_settings
                    (setting_key, value_json, row_version, updated_by, created_at, updated_at)
                VALUES ($key, $valueJson, 1, $actor, $now, $now);
                """,
                cancellationToken,
                ("$key", key), ("$valueJson", valueJson), ("$actor", actor), ("$now", now));
        }
        else
        {
            if (request.RowVersion != current.RowVersion)
            {
                throw new ConcurrencyConflictException("The setting was changed by another user. Reload it and try again.");
            }

            var changed = await ExecuteNonQueryAsync(
                connection,
                transaction,
                """
                UPDATE app_settings
                SET value_json = $valueJson,
                    row_version = row_version + 1,
                    updated_by = $actor,
                    updated_at = $now
                WHERE setting_key = $key AND row_version = $rowVersion;
                """,
                cancellationToken,
                ("$key", key), ("$valueJson", valueJson), ("$actor", actor),
                ("$now", now), ("$rowVersion", current.RowVersion));
            if (changed == 0)
            {
                throw new ConcurrencyConflictException("The setting was changed by another user. Reload it and try again.");
            }
        }

        await WriteAuditAsync(
            connection,
            transaction,
            "setting",
            key,
            current is null ? "created" : "updated",
            actor,
            new { value = request.Value, rowVersion = request.RowVersion },
            cancellationToken);
        await transaction.CommitAsync(cancellationToken);
        return await GetSettingAsync(key, cancellationToken);
    }

    public async Task<IReadOnlyList<FirmwareHistoryRecord>> ListFirmwareHistoryAsync(
        int limit,
        CancellationToken cancellationToken = default)
    {
        limit = Math.Clamp(limit, 1, 1000);
        await using var connection = await OpenConnectionAsync(cancellationToken);
        await using var command = connection.CreateCommand();
        command.CommandText =
            """
            SELECT h.job_id, h.batch_id, h.operation_type, h.group_name, h.receiver_ip,
                   h.receiver_name, h.receiver_model, h.target_file, h.started_at,
                   h.completed_at, h.result, h.final_firmware_version, h.errors_json,
                   h.row_version, h.created_by, h.updated_by, h.created_at, h.updated_at,
                   c.id, c.file_name, c.size_bytes, c.sha256, c.firmware_version, c.created_at
            FROM firmware_update_history h
            LEFT JOIN clone_files c ON c.id = h.clone_file_id
            WHERE h.deleted_at IS NULL
            ORDER BY h.started_at DESC
            LIMIT $limit;
            """;
        command.Parameters.AddWithValue("$limit", limit);
        var records = new List<FirmwareHistoryRecord>();
        await using var reader = await command.ExecuteReaderAsync(cancellationToken);
        while (await reader.ReadAsync(cancellationToken))
        {
            records.Add(ReadFirmwareHistory(reader));
        }

        return records;
    }

    public async Task<FirmwareHistoryRecord> GetFirmwareHistoryAsync(
        string jobId,
        CancellationToken cancellationToken = default)
    {
        jobId = NormalizeHistoryId(jobId);
        await using var connection = await OpenConnectionAsync(cancellationToken);
        await using var command = connection.CreateCommand();
        command.CommandText =
            """
            SELECT h.job_id, h.batch_id, h.operation_type, h.group_name, h.receiver_ip,
                   h.receiver_name, h.receiver_model, h.target_file, h.started_at,
                   h.completed_at, h.result, h.final_firmware_version, h.errors_json,
                   h.row_version, h.created_by, h.updated_by, h.created_at, h.updated_at,
                   c.id, c.file_name, c.size_bytes, c.sha256, c.firmware_version, c.created_at
            FROM firmware_update_history h
            LEFT JOIN clone_files c ON c.id = h.clone_file_id
            WHERE h.job_id = $jobId AND h.deleted_at IS NULL;
            """;
        command.Parameters.AddWithValue("$jobId", jobId);
        await using var reader = await command.ExecuteReaderAsync(cancellationToken);
        return await reader.ReadAsync(cancellationToken)
            ? ReadFirmwareHistory(reader)
            : throw new EntityNotFoundException($"Firmware history entry {jobId} was not found.");
    }

    public async Task<FirmwareHistoryRecord> UpsertFirmwareHistoryAsync(
        string jobId,
        UpsertFirmwareHistoryRequest request,
        string actor,
        CancellationToken cancellationToken = default)
    {
        var value = RequestValidator.Normalize(jobId, request);
        actor = NormalizeActor(actor);
        var now = UtcNowText();
        var errorsJson = JsonSerializer.Serialize(value.Errors);
        await using var connection = await OpenConnectionAsync(cancellationToken);
        await using var transaction = (SqliteTransaction)await connection.BeginTransactionAsync(cancellationToken);

        await using var existsCommand = connection.CreateCommand();
        existsCommand.Transaction = transaction;
        existsCommand.CommandText = "SELECT EXISTS(SELECT 1 FROM firmware_update_history WHERE job_id = $jobId);";
        existsCommand.Parameters.AddWithValue("$jobId", value.JobId);
        var existed = Convert.ToInt64(await existsCommand.ExecuteScalarAsync(cancellationToken), CultureInfo.InvariantCulture) == 1;
        if (existed)
        {
            await using var deletedCommand = connection.CreateCommand();
            deletedCommand.Transaction = transaction;
            deletedCommand.CommandText = "SELECT deleted_at FROM firmware_update_history WHERE job_id = $jobId;";
            deletedCommand.Parameters.AddWithValue("$jobId", value.JobId);
            var deletedAt = await deletedCommand.ExecuteScalarAsync(cancellationToken);
            if (deletedAt is not null and not DBNull)
            {
                throw new ConcurrencyConflictException("This firmware history entry was deleted centrally and cannot be restored from a local cache.");
            }
        }

        var changed = await ExecuteNonQueryAsync(
            connection,
            transaction,
            """
            INSERT INTO firmware_update_history
                (job_id, batch_id, operation_type, group_name, receiver_ip, receiver_name,
                 receiver_model, target_file, started_at, completed_at, result,
                 final_firmware_version, errors_json, row_version, created_by, updated_by,
                 created_at, updated_at)
            VALUES
                ($jobId, $batchId, $operationType, $groupName, $receiverIp, $receiverName,
                 $receiverModel, $targetFile, $startedAt, $completedAt, $result,
                 $finalFirmwareVersion, $errorsJson, 1, $actor, $actor, $now, $now)
            ON CONFLICT(job_id) DO UPDATE SET
                batch_id = excluded.batch_id,
                operation_type = excluded.operation_type,
                group_name = excluded.group_name,
                receiver_ip = excluded.receiver_ip,
                receiver_name = excluded.receiver_name,
                receiver_model = excluded.receiver_model,
                target_file = excluded.target_file,
                started_at = excluded.started_at,
                completed_at = excluded.completed_at,
                result = excluded.result,
                final_firmware_version = excluded.final_firmware_version,
                errors_json = excluded.errors_json,
                row_version = firmware_update_history.row_version + 1,
                updated_by = excluded.updated_by,
                updated_at = excluded.updated_at
            WHERE firmware_update_history.batch_id IS NOT excluded.batch_id
               OR firmware_update_history.operation_type IS NOT excluded.operation_type
               OR firmware_update_history.group_name IS NOT excluded.group_name
               OR firmware_update_history.receiver_ip IS NOT excluded.receiver_ip
               OR firmware_update_history.receiver_name IS NOT excluded.receiver_name
               OR firmware_update_history.receiver_model IS NOT excluded.receiver_model
               OR firmware_update_history.target_file IS NOT excluded.target_file
               OR firmware_update_history.started_at IS NOT excluded.started_at
               OR firmware_update_history.completed_at IS NOT excluded.completed_at
               OR firmware_update_history.result IS NOT excluded.result
               OR firmware_update_history.final_firmware_version IS NOT excluded.final_firmware_version
               OR firmware_update_history.errors_json IS NOT excluded.errors_json;
            """,
            cancellationToken,
            ("$jobId", value.JobId), ("$batchId", value.BatchId),
            ("$operationType", value.OperationType), ("$groupName", value.GroupName),
            ("$receiverIp", value.Ip), ("$receiverName", value.ReceiverName),
            ("$receiverModel", value.Model), ("$targetFile", value.TargetFile),
            ("$startedAt", FormatTimestamp(value.StartedAt)),
            ("$completedAt", value.CompletedAt.HasValue ? FormatTimestamp(value.CompletedAt.Value) : null),
            ("$result", value.Result), ("$finalFirmwareVersion", value.FinalFirmwareVersion),
            ("$errorsJson", errorsJson), ("$actor", actor), ("$now", now));

        if (changed > 0)
        {
            await WriteAuditAsync(
                connection,
                transaction,
                "firmware-history",
                value.JobId,
                existed ? "updated" : "created",
                actor,
                new { value.Ip, value.Result, value.TargetFile },
                cancellationToken);
        }

        await transaction.CommitAsync(cancellationToken);
        return await GetFirmwareHistoryAsync(value.JobId, cancellationToken);
    }

    public async Task<CloneFileSummary> SaveFirmwareCloneFileAsync(
        string jobId,
        string fileName,
        Stream contents,
        long? contentLength,
        string actor,
        CancellationToken cancellationToken = default)
    {
        jobId = NormalizeHistoryId(jobId);
        fileName = NormalizeCloneFileName(fileName);
        actor = NormalizeActor(actor);
        if (contentLength is <= 0 or > MaximumCloneFileSizeBytes)
        {
            throw new RegistryValidationException($"Clone File must contain 1-{MaximumCloneFileSizeBytes} bytes.");
        }

        var incomingDirectory = Path.Combine(_options.CloneFilesPath, ".incoming");
        Directory.CreateDirectory(incomingDirectory);
        var temporaryPath = Path.Combine(incomingDirectory, $"{Guid.NewGuid():N}.tmp");
        var cloneFileId = Guid.NewGuid().ToString("D");
        var relativePath = $"firmware-history/{cloneFileId.Replace("-", string.Empty, StringComparison.Ordinal)}/{fileName}";
        var finalPath = ResolveCloneFilePath(relativePath);
        string sha256;
        long sizeBytes = 0;

        try
        {
            await using (var destination = new FileStream(
                temporaryPath,
                FileMode.CreateNew,
                FileAccess.Write,
                FileShare.None,
                81920,
                FileOptions.Asynchronous | FileOptions.SequentialScan))
            using (var hash = IncrementalHash.CreateHash(HashAlgorithmName.SHA256))
            {
                var buffer = new byte[81920];
                int bytesRead;
                while ((bytesRead = await contents.ReadAsync(buffer, cancellationToken)) > 0)
                {
                    sizeBytes += bytesRead;
                    if (sizeBytes > MaximumCloneFileSizeBytes)
                    {
                        throw new RegistryValidationException($"Clone File cannot exceed {MaximumCloneFileSizeBytes} bytes.");
                    }

                    hash.AppendData(buffer, 0, bytesRead);
                    await destination.WriteAsync(buffer.AsMemory(0, bytesRead), cancellationToken);
                }

                sha256 = Convert.ToHexString(hash.GetHashAndReset()).ToLowerInvariant();
            }

            if (sizeBytes == 0)
            {
                throw new RegistryValidationException("Clone File cannot be empty.");
            }

            var cloneText = await File.ReadAllTextAsync(temporaryPath, cancellationToken);
            if (!IsValidCloneFile(cloneText))
            {
                throw new RegistryValidationException("Clone File does not contain a complete receiver configuration.");
            }

            Directory.CreateDirectory(Path.GetDirectoryName(finalPath)!);
            File.Move(temporaryPath, finalPath);
        }
        catch
        {
            File.Delete(temporaryPath);
            throw;
        }

        string? previousRelativePath = null;
        string? previousCloneFileId = null;
        try
        {
            var now = UtcNowText();
            await using var connection = await OpenConnectionAsync(cancellationToken);
            await using var transaction = (SqliteTransaction)await connection.BeginTransactionAsync(cancellationToken);
            string receiverIp;
            string? firmwareVersion;

            await using (var historyCommand = connection.CreateCommand())
            {
                historyCommand.Transaction = transaction;
                historyCommand.CommandText =
                    """
                    SELECT h.receiver_ip, h.final_firmware_version, h.clone_file_id, c.relative_path
                    FROM firmware_update_history h
                    LEFT JOIN clone_files c ON c.id = h.clone_file_id
                    WHERE h.job_id = $jobId AND h.deleted_at IS NULL;
                    """;
                historyCommand.Parameters.AddWithValue("$jobId", jobId);
                await using var reader = await historyCommand.ExecuteReaderAsync(cancellationToken);
                if (!await reader.ReadAsync(cancellationToken))
                {
                    throw new EntityNotFoundException($"Firmware history entry {jobId} was not found.");
                }

                receiverIp = reader.GetString(0);
                firmwareVersion = GetNullableString(reader, 1);
                previousCloneFileId = GetNullableString(reader, 2);
                previousRelativePath = GetNullableString(reader, 3);
            }

            await ExecuteNonQueryAsync(
                connection,
                transaction,
                """
                INSERT INTO clone_files
                    (id, receiver_id, file_name, relative_path, size_bytes, sha256,
                     firmware_version, description, created_by, created_at)
                VALUES
                    ($id, (SELECT id FROM receivers WHERE ip_address = $receiverIp LIMIT 1),
                     $fileName, $relativePath, $sizeBytes, $sha256, $firmwareVersion,
                     $description, $actor, $now);
                """,
                cancellationToken,
                ("$id", cloneFileId), ("$receiverIp", receiverIp), ("$fileName", fileName),
                ("$relativePath", relativePath), ("$sizeBytes", sizeBytes), ("$sha256", sha256),
                ("$firmwareVersion", firmwareVersion),
                ("$description", $"Configuration backup for firmware job {jobId}"),
                ("$actor", actor), ("$now", now));
            await ExecuteNonQueryAsync(
                connection,
                transaction,
                """
                UPDATE firmware_update_history
                SET clone_file_id = $cloneFileId,
                    row_version = row_version + 1,
                    updated_by = $actor,
                    updated_at = $now
                WHERE job_id = $jobId;
                """,
                cancellationToken,
                ("$cloneFileId", cloneFileId), ("$actor", actor), ("$now", now), ("$jobId", jobId));
            if (previousCloneFileId is not null)
            {
                await ExecuteNonQueryAsync(
                    connection,
                    transaction,
                    "DELETE FROM clone_files WHERE id = $id;",
                    cancellationToken,
                    ("$id", previousCloneFileId));
            }

            await WriteAuditAsync(
                connection,
                transaction,
                "clone-file",
                cloneFileId,
                previousCloneFileId is null ? "created" : "replaced",
                actor,
                new { jobId, fileName, sizeBytes, sha256 },
                cancellationToken);
            await transaction.CommitAsync(cancellationToken);
        }
        catch
        {
            File.Delete(finalPath);
            throw;
        }

        DeleteStoredCloneFile(previousRelativePath);
        return (await GetFirmwareHistoryAsync(jobId, cancellationToken)).CloneFile
            ?? throw new InvalidOperationException("Stored Clone File metadata could not be loaded.");
    }

    public async Task<CloneFileDownload> GetFirmwareCloneFileAsync(
        string jobId,
        CancellationToken cancellationToken = default)
    {
        jobId = NormalizeHistoryId(jobId);
        await using var connection = await OpenConnectionAsync(cancellationToken);
        await using var command = connection.CreateCommand();
        command.CommandText =
            """
            SELECT c.relative_path, c.file_name, c.size_bytes, c.sha256
            FROM firmware_update_history h
            INNER JOIN clone_files c ON c.id = h.clone_file_id
            WHERE h.job_id = $jobId AND h.deleted_at IS NULL;
            """;
        command.Parameters.AddWithValue("$jobId", jobId);
        await using var reader = await command.ExecuteReaderAsync(cancellationToken);
        if (!await reader.ReadAsync(cancellationToken))
        {
            throw new EntityNotFoundException($"Clone File for firmware history entry {jobId} was not found.");
        }

        var fullPath = ResolveCloneFilePath(reader.GetString(0));
        if (!File.Exists(fullPath))
        {
            throw new EntityNotFoundException($"Stored Clone File for firmware history entry {jobId} was not found.");
        }

        return new CloneFileDownload(fullPath, reader.GetString(1), reader.GetInt64(2), reader.GetString(3));
    }

    public async Task DeleteFirmwareHistoryAsync(
        string jobId,
        string actor,
        CancellationToken cancellationToken = default)
    {
        jobId = NormalizeHistoryId(jobId);
        actor = NormalizeActor(actor);
        string? cloneFileId;
        string? relativePath;
        await using var connection = await OpenConnectionAsync(cancellationToken);
        await using var transaction = (SqliteTransaction)await connection.BeginTransactionAsync(cancellationToken);
        await using (var command = connection.CreateCommand())
        {
            command.Transaction = transaction;
            command.CommandText =
                """
                SELECT h.clone_file_id, c.relative_path
                FROM firmware_update_history h
                LEFT JOIN clone_files c ON c.id = h.clone_file_id
                WHERE h.job_id = $jobId AND h.deleted_at IS NULL;
                """;
            command.Parameters.AddWithValue("$jobId", jobId);
            await using var reader = await command.ExecuteReaderAsync(cancellationToken);
            if (!await reader.ReadAsync(cancellationToken))
            {
                throw new EntityNotFoundException($"Firmware history entry {jobId} was not found.");
            }

            cloneFileId = GetNullableString(reader, 0);
            relativePath = GetNullableString(reader, 1);
        }

        await ExecuteNonQueryAsync(
            connection,
            transaction,
            """
            UPDATE firmware_update_history
            SET clone_file_id = NULL,
                row_version = row_version + 1,
                updated_by = $actor,
                updated_at = $now,
                deleted_by = $actor,
                deleted_at = $now
            WHERE job_id = $jobId AND deleted_at IS NULL;
            """,
            cancellationToken,
            ("$jobId", jobId), ("$actor", actor), ("$now", UtcNowText()));
        if (cloneFileId is not null)
        {
            await ExecuteNonQueryAsync(
                connection,
                transaction,
                "DELETE FROM clone_files WHERE id = $id;",
                cancellationToken,
                ("$id", cloneFileId));
        }

        await WriteAuditAsync(
            connection,
            transaction,
            "firmware-history",
            jobId,
            "deleted",
            actor,
            new { cloneFileId },
            cancellationToken);
        await transaction.CommitAsync(cancellationToken);
        DeleteStoredCloneFile(relativePath);
    }

    public async Task<DatabaseStatus> GetStatusAsync(CancellationToken cancellationToken = default)
    {
        await using var connection = await OpenConnectionAsync(cancellationToken);
        var sqliteVersion = await ExecuteScalarStringAsync(connection, "SELECT sqlite_version();", cancellationToken);
        var integrity = await ExecuteScalarStringAsync(connection, "PRAGMA quick_check;", cancellationToken);
        var schemaVersion = await ExecuteScalarLongAsync(connection, "SELECT COALESCE(MAX(version), 0) FROM _schema_migrations;", cancellationToken);
        var receiverCount = await ExecuteScalarLongAsync(connection, "SELECT COUNT(*) FROM receivers;", cancellationToken);
        return new DatabaseStatus(sqliteVersion, checked((int)schemaVersion), receiverCount, integrity);
    }

    public async Task<BackupResult> CreateBackupAsync(CancellationToken cancellationToken = default)
    {
        Directory.CreateDirectory(_options.BackupPath);
        var timestamp = DateTimeOffset.UtcNow;
        var fileName = $"qa-receiver-registry-{timestamp:yyyyMMdd-HHmmss-fff}.db";
        var fullPath = Path.Combine(_options.BackupPath, fileName);

        await using var source = await OpenConnectionAsync(cancellationToken);
        var destinationConnectionString = new SqliteConnectionStringBuilder
        {
            DataSource = fullPath,
            Mode = SqliteOpenMode.ReadWriteCreate,
        }.ToString();
        await using var destination = new SqliteConnection(destinationConnectionString);
        await destination.OpenAsync(cancellationToken);
        source.BackupDatabase(destination);

        var integrity = await ExecuteScalarStringAsync(destination, "PRAGMA quick_check;", cancellationToken);
        if (!string.Equals(integrity, "ok", StringComparison.OrdinalIgnoreCase))
        {
            destination.Dispose();
            File.Delete(fullPath);
            throw new InvalidOperationException($"Backup integrity check failed: {integrity}");
        }

        PruneBackups();
        var info = new FileInfo(fullPath);
        _logger.LogInformation("Created database backup {BackupPath}", fullPath);
        return new BackupResult(fileName, fullPath, info.Length, timestamp);
    }

    private async Task<SqliteConnection> OpenConnectionAsync(CancellationToken cancellationToken)
    {
        var connection = new SqliteConnection(_connectionString);
        await connection.OpenAsync(cancellationToken);
        return connection;
    }

    private static FirmwareHistoryRecord ReadFirmwareHistory(SqliteDataReader reader)
    {
        var errors = JsonSerializer.Deserialize<string[]>(reader.GetString(12)) ?? [];
        CloneFileSummary? cloneFile = null;
        if (!reader.IsDBNull(18))
        {
            cloneFile = new CloneFileSummary(
                reader.GetString(18), reader.GetString(19), reader.GetInt64(20), reader.GetString(21),
                GetNullableString(reader, 22), ParseTimestamp(reader.GetString(23)));
        }

        return new FirmwareHistoryRecord(
            reader.GetString(0), GetNullableString(reader, 1), GetNullableString(reader, 2),
            GetNullableString(reader, 3), reader.GetString(4), GetNullableString(reader, 5),
            reader.GetString(6), reader.GetString(7), FormatUnixTimestamp(ParseTimestamp(reader.GetString(8))),
            reader.IsDBNull(9) ? null : FormatUnixTimestamp(ParseTimestamp(reader.GetString(9))), reader.GetString(10),
            GetNullableString(reader, 11), cloneFile, errors, reader.GetInt64(13), reader.GetString(14),
            reader.GetString(15), ParseTimestamp(reader.GetString(16)), ParseTimestamp(reader.GetString(17)));
    }

    private static string NormalizeHistoryId(string jobId)
    {
        jobId = jobId?.Trim() ?? string.Empty;
        if (jobId.Length is < 1 or > 120)
        {
            throw new RegistryValidationException("jobId must contain 1-120 characters.");
        }

        return jobId;
    }

    private static string NormalizeCloneFileName(string fileName)
    {
        fileName = fileName?.Trim() ?? string.Empty;
        if (fileName.Length is < 1 or > 200 || fileName != Path.GetFileName(fileName) ||
            fileName.IndexOfAny(Path.GetInvalidFileNameChars()) >= 0)
        {
            throw new RegistryValidationException("fileName must be a valid file name containing 1-200 characters.");
        }

        return fileName;
    }

    private string ResolveCloneFilePath(string relativePath)
    {
        var root = Path.GetFullPath(_options.CloneFilesPath);
        var rootPrefix = root.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar) + Path.DirectorySeparatorChar;
        var fullPath = Path.GetFullPath(Path.Combine(root, relativePath.Replace('/', Path.DirectorySeparatorChar)));
        if (!fullPath.StartsWith(rootPrefix, StringComparison.OrdinalIgnoreCase))
        {
            throw new InvalidOperationException("Stored Clone File path is outside the configured directory.");
        }

        return fullPath;
    }

    private void DeleteStoredCloneFile(string? relativePath)
    {
        if (string.IsNullOrWhiteSpace(relativePath))
        {
            return;
        }

        var fullPath = ResolveCloneFilePath(relativePath);
        try
        {
            File.Delete(fullPath);
            var directory = Path.GetDirectoryName(fullPath);
            if (directory is not null && Directory.Exists(directory) && !Directory.EnumerateFileSystemEntries(directory).Any())
            {
                Directory.Delete(directory);
            }
        }
        catch (IOException exception)
        {
            _logger.LogWarning(exception, "Could not delete stored Clone File {CloneFilePath}", fullPath);
        }
    }

    private static bool IsValidCloneFile(string contents)
    {
        var trimmed = contents.Trim();
        return trimmed.Length > 0 &&
               !trimmed.Contains("<fail>1</fail>", StringComparison.OrdinalIgnoreCase) &&
               trimmed.StartsWith('<') &&
               trimmed.Contains("</CloneConfigData>", StringComparison.Ordinal);
    }

    private static string FormatTimestamp(DateTimeOffset value) =>
        value.ToUniversalTime().ToString("O", CultureInfo.InvariantCulture);

    private static string FormatUnixTimestamp(DateTimeOffset value) =>
        value.ToUnixTimeSeconds().ToString(CultureInfo.InvariantCulture);

    private async Task<bool> IsMigrationAppliedAsync(SqliteConnection connection, int version, CancellationToken cancellationToken)
    {
        await using var command = connection.CreateCommand();
        command.CommandText = "SELECT EXISTS(SELECT 1 FROM _schema_migrations WHERE version = $version);";
        command.Parameters.AddWithValue("$version", version);
        return Convert.ToInt64(await command.ExecuteScalarAsync(cancellationToken), CultureInfo.InvariantCulture) == 1;
    }

    private static async Task<ReceiverRecord?> FindReceiverAsync(
        SqliteConnection connection,
        SqliteTransaction? transaction,
        string id,
        CancellationToken cancellationToken)
    {
        await using var command = connection.CreateCommand();
        command.Transaction = transaction;
        command.CommandText =
            """
            SELECT id, ip_address, name, model, serial_number, location, notes, enabled,
                   row_version, created_at, updated_at
            FROM receivers WHERE id = $id;
            """;
        command.Parameters.AddWithValue("$id", id);
        await using var reader = await command.ExecuteReaderAsync(cancellationToken);
        return await reader.ReadAsync(cancellationToken) ? ReadReceiver(reader, []) : null;
    }

    private static ReceiverRecord ReadReceiver(SqliteDataReader reader, IReadOnlyList<GroupSummary> groups)
    {
        return new ReceiverRecord(
            reader.GetString(0), reader.GetString(1), GetNullableString(reader, 2),
            GetNullableString(reader, 3), GetNullableString(reader, 4), GetNullableString(reader, 5),
            GetNullableString(reader, 6), reader.GetInt64(7) == 1, reader.GetInt64(8),
            ParseTimestamp(reader.GetString(9)), ParseTimestamp(reader.GetString(10)), groups);
    }

    private static async Task<SettingRecord?> FindSettingAsync(
        SqliteConnection connection,
        SqliteTransaction? transaction,
        string key,
        CancellationToken cancellationToken)
    {
        await using var command = connection.CreateCommand();
        command.Transaction = transaction;
        command.CommandText =
            """
            SELECT setting_key, value_json, row_version, updated_by, created_at, updated_at
            FROM app_settings
            WHERE setting_key = $key;
            """;
        command.Parameters.AddWithValue("$key", key);
        await using var reader = await command.ExecuteReaderAsync(cancellationToken);
        return await reader.ReadAsync(cancellationToken) ? ReadSetting(reader) : null;
    }

    private static SettingRecord ReadSetting(SqliteDataReader reader)
    {
        using var document = JsonDocument.Parse(reader.GetString(1));
        return new SettingRecord(
            reader.GetString(0),
            document.RootElement.Clone(),
            reader.GetInt64(2),
            reader.GetString(3),
            ParseTimestamp(reader.GetString(4)),
            ParseTimestamp(reader.GetString(5)));
    }

    private static async Task<Dictionary<string, IReadOnlyList<GroupSummary>>> LoadMembershipsAsync(
        SqliteConnection connection,
        IEnumerable<string> receiverIds,
        CancellationToken cancellationToken)
    {
        var ids = receiverIds.Distinct().ToArray();
        if (ids.Length == 0)
        {
            return [];
        }

        await using var command = connection.CreateCommand();
        var parameters = ids.Select((id, index) => (Name: $"$id{index}", Value: id)).ToArray();
        command.CommandText =
            $"""
            SELECT m.receiver_id, g.id, g.code, g.name, g.description, g.is_system,
                   g.enabled, g.row_version, m.is_primary
            FROM receiver_group_memberships m
            INNER JOIN receiver_groups g ON g.id = m.group_id
            WHERE m.receiver_id IN ({string.Join(", ", parameters.Select(item => item.Name))})
            ORDER BY m.is_primary DESC, g.id;
            """;
        foreach (var parameter in parameters)
        {
            command.Parameters.AddWithValue(parameter.Name, parameter.Value);
        }

        var result = new Dictionary<string, List<GroupSummary>>(StringComparer.OrdinalIgnoreCase);
        await using var reader = await command.ExecuteReaderAsync(cancellationToken);
        while (await reader.ReadAsync(cancellationToken))
        {
            var receiverId = reader.GetString(0);
            if (!result.TryGetValue(receiverId, out var memberships))
            {
                memberships = [];
                result.Add(receiverId, memberships);
            }

            memberships.Add(new GroupSummary(
                reader.GetInt64(1), reader.GetString(2), reader.GetString(3),
                GetNullableString(reader, 4), reader.GetInt64(5) == 1,
                reader.GetInt64(6) == 1, reader.GetInt64(7), reader.GetInt64(8) == 1));
        }

        return result.ToDictionary(pair => pair.Key, pair => (IReadOnlyList<GroupSummary>)pair.Value, StringComparer.OrdinalIgnoreCase);
    }

    private static GroupSummary ReadGroup(SqliteDataReader reader, bool isPrimary)
    {
        return new GroupSummary(
            reader.GetInt64(0), reader.GetString(1), reader.GetString(2),
            GetNullableString(reader, 3), reader.GetInt64(4) == 1,
            reader.GetInt64(5) == 1, reader.GetInt64(6), isPrimary);
    }

    private static async Task ValidateGroupsAsync(
        SqliteConnection connection,
        SqliteTransaction transaction,
        IReadOnlyList<long> groupIds,
        CancellationToken cancellationToken)
    {
        if (groupIds.Count == 0)
        {
            return;
        }

        await using var command = connection.CreateCommand();
        command.Transaction = transaction;
        var names = groupIds.Select((_, index) => $"$group{index}").ToArray();
        command.CommandText = $"SELECT COUNT(*) FROM receiver_groups WHERE enabled = 1 AND id IN ({string.Join(", ", names)});";
        for (var index = 0; index < groupIds.Count; index++)
        {
            command.Parameters.AddWithValue(names[index], groupIds[index]);
        }

        var count = Convert.ToInt64(await command.ExecuteScalarAsync(cancellationToken), CultureInfo.InvariantCulture);
        if (count != groupIds.Count)
        {
            throw new RegistryValidationException("One or more groupIds do not refer to enabled groups.");
        }
    }

    private static async Task ValidateSystemGroupUpdateAsync(
        SqliteConnection connection,
        SqliteTransaction transaction,
        long id,
        NormalizedGroup value,
        CancellationToken cancellationToken)
    {
        await using var command = connection.CreateCommand();
        command.Transaction = transaction;
        command.CommandText = "SELECT code, is_system FROM receiver_groups WHERE id = $id;";
        command.Parameters.AddWithValue("$id", id);
        await using var reader = await command.ExecuteReaderAsync(cancellationToken);
        if (!await reader.ReadAsync(cancellationToken))
        {
            throw new EntityNotFoundException($"Group {id} was not found.");
        }

        var currentCode = reader.GetString(0);
        var isSystem = reader.GetInt64(1) == 1;
        if (isSystem && (!value.Enabled || !string.Equals(currentCode, value.Code, StringComparison.OrdinalIgnoreCase)))
        {
            throw new RegistryValidationException("System groups cannot be disabled or have their code changed.");
        }
    }

    private static async Task ReplaceMembershipsAsync(
        SqliteConnection connection,
        SqliteTransaction transaction,
        string receiverId,
        IReadOnlyList<long> groupIds,
        long? primaryGroupId,
        string now,
        CancellationToken cancellationToken)
    {
        await ExecuteNonQueryAsync(
            connection,
            transaction,
            "DELETE FROM receiver_group_memberships WHERE receiver_id = $receiverId;",
            cancellationToken,
            ("$receiverId", receiverId));
        foreach (var groupId in groupIds)
        {
            await ExecuteNonQueryAsync(
                connection,
                transaction,
                """
                INSERT INTO receiver_group_memberships(receiver_id, group_id, is_primary, created_at)
                VALUES ($receiverId, $groupId, $isPrimary, $now);
                """,
                cancellationToken,
                ("$receiverId", receiverId), ("$groupId", groupId),
                ("$isPrimary", groupId == primaryGroupId ? 1 : 0), ("$now", now));
        }
    }

    private static async Task WriteAuditAsync(
        SqliteConnection connection,
        SqliteTransaction transaction,
        string entityType,
        string entityId,
        string action,
        string actor,
        object payload,
        CancellationToken cancellationToken)
    {
        actor = NormalizeActor(actor);

        await ExecuteNonQueryAsync(
            connection,
            transaction,
            """
            INSERT INTO audit_log(entity_type, entity_id, action, actor, changed_at, payload_json)
            VALUES ($entityType, $entityId, $action, $actor, $changedAt, $payloadJson);
            """,
            cancellationToken,
            ("$entityType", entityType), ("$entityId", entityId), ("$action", action),
            ("$actor", actor), ("$changedAt", UtcNowText()),
            ("$payloadJson", JsonSerializer.Serialize(payload)));
    }

    private static string NormalizeSettingKey(string key)
    {
        key = key?.Trim().ToLowerInvariant() ?? string.Empty;
        if (key.Length is < 2 or > 100 || key.Any(character =>
                !char.IsAsciiLetterOrDigit(character) && character is not '-' and not '_' and not '.'))
        {
            throw new RegistryValidationException(
                "Setting key must contain 2-100 letters, numbers, dots, hyphens, or underscores.");
        }

        return key;
    }

    private static string NormalizeSettingValue(JsonElement value)
    {
        if (value.ValueKind is JsonValueKind.Undefined)
        {
            throw new RegistryValidationException("Setting value is required.");
        }

        var json = value.GetRawText();
        if (json.Length > 100_000)
        {
            throw new RegistryValidationException("Setting value cannot exceed 100000 characters.");
        }

        return json;
    }

    private static string NormalizeActor(string actor)
    {
        actor = string.IsNullOrWhiteSpace(actor) ? "unknown" : actor.Trim();
        return actor.Length <= 200 ? actor : actor[..200];
    }

    private static async Task ThrowNotFoundOrConflictAsync(
        SqliteConnection connection,
        SqliteTransaction transaction,
        string table,
        string id,
        CancellationToken cancellationToken)
    {
        await using var command = connection.CreateCommand();
        command.Transaction = transaction;
        command.CommandText = $"SELECT EXISTS(SELECT 1 FROM {table} WHERE id = $id);";
        command.Parameters.AddWithValue("$id", id);
        var exists = Convert.ToInt64(await command.ExecuteScalarAsync(cancellationToken), CultureInfo.InvariantCulture) == 1;
        if (!exists)
        {
            throw new EntityNotFoundException($"Entity {id} was not found.");
        }

        throw new ConcurrencyConflictException("The record was changed by another user. Reload it and try again.");
    }

    private static async Task<int> ExecuteNonQueryAsync(
        SqliteConnection connection,
        SqliteTransaction? transaction,
        string sql,
        CancellationToken cancellationToken,
        params (string Name, object? Value)[] parameters)
    {
        await using var command = connection.CreateCommand();
        command.Transaction = transaction;
        command.CommandText = sql;
        foreach (var parameter in parameters)
        {
            command.Parameters.AddWithValue(parameter.Name, parameter.Value ?? DBNull.Value);
        }

        return await command.ExecuteNonQueryAsync(cancellationToken);
    }

    private static async Task<string> ExecuteScalarStringAsync(
        SqliteConnection connection,
        string sql,
        CancellationToken cancellationToken)
    {
        await using var command = connection.CreateCommand();
        command.CommandText = sql;
        return Convert.ToString(await command.ExecuteScalarAsync(cancellationToken), CultureInfo.InvariantCulture) ?? string.Empty;
    }

    private static async Task<long> ExecuteScalarLongAsync(
        SqliteConnection connection,
        string sql,
        CancellationToken cancellationToken)
    {
        await using var command = connection.CreateCommand();
        command.CommandText = sql;
        return Convert.ToInt64(await command.ExecuteScalarAsync(cancellationToken), CultureInfo.InvariantCulture);
    }

    private void PruneBackups()
    {
        var backups = new DirectoryInfo(_options.BackupPath)
            .GetFiles("qa-receiver-registry-*.db")
            .OrderByDescending(file => file.CreationTimeUtc)
            .Skip(_options.BackupRetentionCount);
        foreach (var backup in backups)
        {
            try
            {
                backup.Delete();
            }
            catch (IOException exception)
            {
                _logger.LogWarning(exception, "Could not remove expired backup {BackupPath}", backup.FullName);
            }
        }
    }

    private static string? GetNullableString(SqliteDataReader reader, int ordinal)
    {
        return reader.IsDBNull(ordinal) ? null : reader.GetString(ordinal);
    }

    private static string UtcNowText() => DateTimeOffset.UtcNow.ToString("O", CultureInfo.InvariantCulture);

    private static DateTimeOffset ParseTimestamp(string value)
    {
        return DateTimeOffset.Parse(value, CultureInfo.InvariantCulture, DateTimeStyles.AssumeUniversal);
    }

    private sealed class ByteArrayComparer : IComparer<byte[]>
    {
        public static ByteArrayComparer Instance { get; } = new();

        public int Compare(byte[]? left, byte[]? right)
        {
            if (ReferenceEquals(left, right))
            {
                return 0;
            }

            if (left is null)
            {
                return -1;
            }

            if (right is null)
            {
                return 1;
            }

            for (var index = 0; index < Math.Min(left.Length, right.Length); index++)
            {
                var comparison = left[index].CompareTo(right[index]);
                if (comparison != 0)
                {
                    return comparison;
                }
            }

            return left.Length.CompareTo(right.Length);
        }
    }
}
