namespace QAReceiverRegistry.Configuration;

public sealed class RegistryOptions
{
    public const string SectionName = "Registry";

    public string DatabasePath { get; set; } = "data/receivers.db";

    public string CloneFilesPath { get; set; } = "clone-files";

    public string BackupPath { get; set; } = "backups";

    public int BusyTimeoutSeconds { get; set; } = 15;

    public int BackupIntervalHours { get; set; } = 24;

    public int BackupRetentionCount { get; set; } = 14;

    public bool BackupOnStartup { get; set; } = true;

    public void NormalizeAndValidate()
    {
        DatabasePath = ResolvePath(DatabasePath, nameof(DatabasePath));
        CloneFilesPath = ResolvePath(CloneFilesPath, nameof(CloneFilesPath));
        BackupPath = ResolvePath(BackupPath, nameof(BackupPath));

        if (BusyTimeoutSeconds is < 1 or > 120)
        {
            throw new InvalidOperationException("Registry:BusyTimeoutSeconds must be between 1 and 120.");
        }

        if (BackupIntervalHours is < 1 or > 168)
        {
            throw new InvalidOperationException("Registry:BackupIntervalHours must be between 1 and 168.");
        }

        if (BackupRetentionCount is < 1 or > 365)
        {
            throw new InvalidOperationException("Registry:BackupRetentionCount must be between 1 and 365.");
        }
    }

    private static string ResolvePath(string value, string name)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            throw new InvalidOperationException($"Registry:{name} is required.");
        }

        return Path.GetFullPath(value, AppContext.BaseDirectory);
    }
}

public sealed class ServerOptions
{
    public const string SectionName = "Server";

    public string ListenAddress { get; set; } = "127.0.0.1";

    public int Port { get; set; } = 8787;

    public bool UseHttps { get; set; }

    public string CertificateThumbprint { get; set; } = string.Empty;

    public void Validate()
    {
        if (!System.Net.IPAddress.TryParse(ListenAddress, out _))
        {
            throw new InvalidOperationException("Server:ListenAddress must be a valid IP address.");
        }

        if (Port is < 1024 or > 65535)
        {
            throw new InvalidOperationException("Server:Port must be between 1024 and 65535.");
        }

        if (UseHttps && string.IsNullOrWhiteSpace(CertificateThumbprint))
        {
            throw new InvalidOperationException("Server:CertificateThumbprint is required when HTTPS is enabled.");
        }
    }
}
