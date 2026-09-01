using System.Net;
using System.Text.Json;
using Microsoft.Extensions.Logging.EventLog;
using Microsoft.Extensions.Options;
using QAReceiverRegistry.Api;
using QAReceiverRegistry.Configuration;
using QAReceiverRegistry.Data;
using QAReceiverRegistry.Infrastructure;
using QAReceiverRegistry.Services;

var configPath = GetArgumentValue(args, "--config");
var builder = WebApplication.CreateBuilder(args);
builder.Host.UseWindowsService(options => options.ServiceName = "QA Receiver Registry");
if (Environment.UserInteractive)
{
    builder.Logging.ClearProviders();
    builder.Logging.AddConsole();
}
else
{
    builder.Services.Configure<EventLogSettings>(settings =>
    {
        settings.LogName = "Application";
        settings.SourceName = "QA Receiver Registry";
    });
}

if (!string.IsNullOrWhiteSpace(configPath))
{
    builder.Configuration.AddJsonFile(Path.GetFullPath(configPath), optional: false, reloadOnChange: false);
}

var registryOptions = builder.Configuration.GetSection(RegistryOptions.SectionName).Get<RegistryOptions>() ?? new RegistryOptions();
registryOptions.NormalizeAndValidate();
var serverOptions = builder.Configuration.GetSection(ServerOptions.SectionName).Get<ServerOptions>() ?? new ServerOptions();
serverOptions.Validate();

builder.Services.AddSingleton(Options.Create(registryOptions));
builder.Services.AddSingleton(Options.Create(serverOptions));
builder.Services.AddSingleton<RegistryDatabase>();
builder.Services.AddHostedService<BackupWorker>();
builder.Services.ConfigureHttpJsonOptions(options =>
{
    options.SerializerOptions.PropertyNamingPolicy = JsonNamingPolicy.CamelCase;
    options.SerializerOptions.WriteIndented = false;
});

builder.WebHost.ConfigureKestrel(options =>
{
    options.AddServerHeader = false;
    options.Listen(IPAddress.Parse(serverOptions.ListenAddress), serverOptions.Port, listenOptions =>
    {
        if (serverOptions.UseHttps)
        {
            listenOptions.UseHttps(CertificateLoader.LoadFromLocalMachine(serverOptions.CertificateThumbprint));
        }
    });
});

var app = builder.Build();
var database = app.Services.GetRequiredService<RegistryDatabase>();
await database.InitializeAsync();

if (args.Contains("--migrate-only", StringComparer.OrdinalIgnoreCase))
{
    return;
}

if (args.Contains("--backup-only", StringComparer.OrdinalIgnoreCase))
{
    var backup = await database.CreateBackupAsync();
    Console.WriteLine(backup.FullPath);
    return;
}

app.UseMiddleware<ApiExceptionMiddleware>();
app.MapRegistryEndpoints();
await app.RunAsync();

static string? GetArgumentValue(string[] arguments, string name)
{
    for (var index = 0; index < arguments.Length - 1; index++)
    {
        if (string.Equals(arguments[index], name, StringComparison.OrdinalIgnoreCase))
        {
            return arguments[index + 1];
        }
    }

    return null;
}
