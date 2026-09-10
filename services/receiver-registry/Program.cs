using System.Net;
using System.Text.Json;
using System.Threading.RateLimiting;
using Microsoft.Extensions.Logging.EventLog;
using Microsoft.Extensions.FileProviders;
using Microsoft.Extensions.Options;
using Microsoft.AspNetCore.RateLimiting;
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
var featureOptions = builder.Configuration.GetSection(FeatureOptions.SectionName).Get<FeatureOptions>() ?? new FeatureOptions();
featureOptions.NormalizeAndValidate();
var ipPingOptions = builder.Configuration.GetSection(IpPingOptions.SectionName).Get<IpPingOptions>() ?? new IpPingOptions();
ipPingOptions.Validate();

builder.Services.AddSingleton(Options.Create(registryOptions));
builder.Services.AddSingleton(Options.Create(serverOptions));
builder.Services.AddSingleton(Options.Create(featureOptions));
builder.Services.AddSingleton(Options.Create(ipPingOptions));
builder.Services.AddSingleton<RegistryDatabase>();
builder.Services.AddHostedService<BackupWorker>();
builder.Services.AddSingleton<IpPingService>();
builder.Services.AddHostedService(provider => provider.GetRequiredService<IpPingService>());
builder.Services.AddRateLimiter(options =>
{
    options.RejectionStatusCode = StatusCodes.Status429TooManyRequests;
    options.AddPolicy("guest-ip-requests", context =>
        RateLimitPartition.GetFixedWindowLimiter(
            context.Connection.RemoteIpAddress?.ToString() ?? "unknown",
            _ => new FixedWindowRateLimiterOptions
            {
                PermitLimit = 5,
                Window = TimeSpan.FromMinutes(1),
                QueueLimit = 0,
                AutoReplenishment = true,
            }));
});
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
app.UseRateLimiter();
if (featureOptions.GuestPortalEnabled)
{
    var guestRoot = Path.Combine(AppContext.BaseDirectory, "wwwroot", "guest");
    if (!Directory.Exists(guestRoot))
    {
        throw new DirectoryNotFoundException($"Guest portal files were not found: {guestRoot}");
    }

    var guestFiles = new PhysicalFileProvider(guestRoot);
    app.Use(async (context, next) =>
    {
        if (!context.Request.Path.StartsWithSegments("/api") &&
            !context.Request.Path.StartsWithSegments("/health"))
        {
            context.Response.Headers["Content-Security-Policy"] =
                "default-src 'self'; img-src 'self'; style-src 'self'; script-src 'self'; connect-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'";
            context.Response.Headers["X-Content-Type-Options"] = "nosniff";
            context.Response.Headers["X-Frame-Options"] = "DENY";
            context.Response.Headers["Referrer-Policy"] = "no-referrer";
        }

        await next();
    });
    app.UseDefaultFiles(new DefaultFilesOptions
    {
        FileProvider = guestFiles,
    });
    app.UseStaticFiles(new StaticFileOptions
    {
        FileProvider = guestFiles,
    });
    app.UseDefaultFiles(new DefaultFilesOptions
    {
        FileProvider = guestFiles,
        RequestPath = "/guest",
    });
    app.UseStaticFiles(new StaticFileOptions
    {
        FileProvider = guestFiles,
        RequestPath = "/guest",
    });
}
app.MapRegistryEndpoints(featureOptions);
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
