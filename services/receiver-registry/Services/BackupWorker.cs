using Microsoft.Extensions.Options;
using QAReceiverRegistry.Configuration;
using QAReceiverRegistry.Data;

namespace QAReceiverRegistry.Services;

public sealed class BackupWorker(
    RegistryDatabase database,
    IOptions<RegistryOptions> options,
    ILogger<BackupWorker> logger) : BackgroundService
{
    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        if (options.Value.BackupOnStartup)
        {
            await TryCreateBackupAsync(stoppingToken);
        }

        try
        {
            using var timer = new PeriodicTimer(TimeSpan.FromHours(options.Value.BackupIntervalHours));
            while (await timer.WaitForNextTickAsync(stoppingToken))
            {
                await TryCreateBackupAsync(stoppingToken);
            }
        }
        catch (OperationCanceledException) when (stoppingToken.IsCancellationRequested)
        {
            // Normal service shutdown.
        }
    }

    private async Task TryCreateBackupAsync(CancellationToken cancellationToken)
    {
        try
        {
            await database.CreateBackupAsync(cancellationToken);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception exception)
        {
            logger.LogError(exception, "Scheduled database backup failed");
        }
    }
}
