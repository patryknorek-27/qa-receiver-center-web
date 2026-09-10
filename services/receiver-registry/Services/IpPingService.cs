using System.Collections.Concurrent;
using System.Net.NetworkInformation;
using System.Text.Json;
using Microsoft.Extensions.Options;
using QAReceiverRegistry.Configuration;
using QAReceiverRegistry.Data;
using QAReceiverRegistry.Infrastructure;
using QAReceiverRegistry.Models;

namespace QAReceiverRegistry.Services;

public sealed class IpPingService(
    RegistryDatabase database,
    IOptions<FeatureOptions> featureOptions,
    IOptions<IpPingOptions> pingOptions,
    ILogger<IpPingService> logger) : BackgroundService
{
    private static readonly JsonSerializerOptions JsonOptions = new(JsonSerializerDefaults.Web);
    private readonly SemaphoreSlim _scanLock = new(1, 1);

    public async Task<IpPingScanResult> RunScanAsync(
        string actor,
        CancellationToken cancellationToken = default)
    {
        EnsurePingEnabled();

        if (!await _scanLock.WaitAsync(0, cancellationToken))
        {
            throw new ConcurrencyConflictException("An IP ping scan is already running.");
        }

        var startedAt = DateTimeOffset.UtcNow;
        try
        {
            var inventory = await database.ListIpInventoryAsync(cancellationToken);
            var settings = await ReadMonitoringSettingsAsync(cancellationToken);
            var results = new ConcurrentBag<IpPingResult>();
            await Parallel.ForEachAsync(
                inventory,
                new ParallelOptions
                {
                    CancellationToken = cancellationToken,
                    MaxDegreeOfParallelism = pingOptions.Value.MaxParallelism,
                },
                async (record, token) =>
                {
                    var isOnline = await PingAsync(record.IpAddress, settings.TimeoutMilliseconds);
                    token.ThrowIfCancellationRequested();
                    results.Add(new IpPingResult(record.IpAddress, isOnline, DateTimeOffset.UtcNow));
                });

            var orderedResults = results.OrderBy(result => result.IpAddress).ToArray();
            await database.SaveIpPingResultsAsync(orderedResults, cancellationToken);
            var completedAt = DateTimeOffset.UtcNow;
            var onlineCount = orderedResults.Count(result => result.IsOnline);
            logger.LogInformation(
                "IP ping scan completed by {Actor}: {OnlineCount} online and {OfflineCount} offline",
                string.IsNullOrWhiteSpace(actor) ? "unknown" : actor,
                onlineCount,
                orderedResults.Length - onlineCount);
            return new IpPingScanResult(
                orderedResults.Length,
                onlineCount,
                orderedResults.Length - onlineCount,
                startedAt,
                completedAt);
        }
        finally
        {
            _scanLock.Release();
        }
    }

    public async Task<IpPingResult> RunSinglePingAsync(
        string ipAddress,
        string actor,
        CancellationToken cancellationToken = default)
    {
        EnsurePingEnabled();
        ipAddress = RequestValidator.NormalizeLabIpAddress(ipAddress);

        if (!await _scanLock.WaitAsync(0, cancellationToken))
        {
            throw new ConcurrencyConflictException("An IP ping scan is already running.");
        }

        try
        {
            await database.GetIpInventoryAsync(ipAddress, cancellationToken);
            var settings = await ReadMonitoringSettingsAsync(cancellationToken);
            var result = new IpPingResult(
                ipAddress,
                await PingAsync(ipAddress, settings.TimeoutMilliseconds),
                DateTimeOffset.UtcNow);
            await database.SaveIpPingResultsAsync([result], cancellationToken);
            logger.LogInformation(
                "IP {IpAddress} ping completed by {Actor}: {Status}",
                ipAddress,
                string.IsNullOrWhiteSpace(actor) ? "unknown" : actor,
                result.IsOnline ? "online" : "offline");
            return result;
        }
        finally
        {
            _scanLock.Release();
        }
    }

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        if (!featureOptions.Value.IpManagementEnabled || !pingOptions.Value.Enabled)
        {
            return;
        }

        try
        {
            using var timer = new PeriodicTimer(TimeSpan.FromSeconds(15));
            DateTimeOffset? nextScanAt = null;
            IpMonitoringSettings? previousSettings = null;
            while (await timer.WaitForNextTickAsync(stoppingToken))
            {
                try
                {
                    var settings = await ReadMonitoringSettingsAsync(stoppingToken);
                    if (!settings.AutomaticScanEnabled)
                    {
                        nextScanAt = null;
                        previousSettings = settings;
                        continue;
                    }

                    if (previousSettings != settings || nextScanAt is null)
                    {
                        nextScanAt = DateTimeOffset.UtcNow.AddMinutes(settings.IntervalMinutes);
                        previousSettings = settings;
                        continue;
                    }

                    if (DateTimeOffset.UtcNow < nextScanAt.Value)
                    {
                        continue;
                    }

                    await RunScanAsync("scheduled-ping", stoppingToken);
                    nextScanAt = DateTimeOffset.UtcNow.AddMinutes(settings.IntervalMinutes);
                }
                catch (OperationCanceledException) when (stoppingToken.IsCancellationRequested)
                {
                    throw;
                }
                catch (Exception exception)
                {
                    logger.LogError(exception, "Scheduled IP ping scan failed");
                    nextScanAt = DateTimeOffset.UtcNow.AddMinutes(1);
                }
            }
        }
        catch (OperationCanceledException) when (stoppingToken.IsCancellationRequested)
        {
            // Normal service shutdown.
        }
    }

    private void EnsurePingEnabled()
    {
        if (!featureOptions.Value.IpManagementEnabled || !pingOptions.Value.Enabled)
        {
            throw new RegistryValidationException("IP ping is not enabled.");
        }
    }

    private async Task<IpMonitoringSettings> ReadMonitoringSettingsAsync(CancellationToken cancellationToken)
    {
        var setting = await database.GetSettingAsync("ip-monitoring", cancellationToken);
        var settings = setting.Value.Deserialize<IpMonitoringSettings>(JsonOptions)
            ?? throw new RegistryValidationException("IP monitoring settings are missing.");
        return RequestValidator.Normalize(settings);
    }

    private static async Task<bool> PingAsync(string ipAddress, int timeoutMilliseconds)
    {
        try
        {
            using var ping = new Ping();
            var reply = await ping.SendPingAsync(ipAddress, timeoutMilliseconds);
            return reply.Status == IPStatus.Success;
        }
        catch (PingException)
        {
            return false;
        }
    }
}
