using System.Reflection;
using System.Text.Json;
using Microsoft.AspNetCore.RateLimiting;
using QAReceiverRegistry.Configuration;
using QAReceiverRegistry.Data;
using QAReceiverRegistry.Infrastructure;
using QAReceiverRegistry.Models;
using QAReceiverRegistry.Services;

namespace QAReceiverRegistry.Api;

public static class RegistryEndpoints
{
    public static IEndpointRouteBuilder MapRegistryEndpoints(
        this IEndpointRouteBuilder endpoints,
        FeatureOptions? featureOptions = null)
    {
        featureOptions ??= new FeatureOptions();
        endpoints.MapGet("/health", GetHealthAsync).AllowAnonymous();

        var api = endpoints.MapGroup("/api/v1");
        api.MapGet("/receivers", async (bool? includeDisabled, RegistryDatabase database, CancellationToken token) =>
            Results.Ok(await database.ListReceiversAsync(includeDisabled ?? false, token)));
        api.MapGet("/receivers/{id}", async (string id, RegistryDatabase database, CancellationToken token) =>
            Results.Ok(await database.GetReceiverAsync(id, token)));
        api.MapPost("/receivers", async (
            CreateReceiverRequest request,
            HttpContext context,
            RegistryDatabase database,
            CancellationToken token) =>
        {
            var receiver = await database.CreateReceiverAsync(request, GetActor(context), token);
            return Results.Created($"/api/v1/receivers/{receiver.Id}", receiver);
        });
        api.MapPut("/receivers/{id}", async (
            string id,
            UpdateReceiverRequest request,
            HttpContext context,
            RegistryDatabase database,
            CancellationToken token) =>
            Results.Ok(await database.UpdateReceiverAsync(id, request, GetActor(context), token)));
        api.MapDelete("/receivers/{id}", async (
            string id,
            long rowVersion,
            HttpContext context,
            RegistryDatabase database,
            CancellationToken token) =>
        {
            await database.DisableReceiverAsync(id, rowVersion, GetActor(context), token);
            return Results.NoContent();
        });

        if (featureOptions.IpManagementEnabled)
        {
            var ipAdminApi = api.MapGroup("/ip-addresses");

            ipAdminApi.MapGet("", async (RegistryDatabase database, CancellationToken token) =>
                Results.Ok(await database.ListIpInventoryAsync(token)));
            ipAdminApi.MapPost("/ping", async (
                HttpContext context,
                IpPingService pingService,
                CancellationToken token) =>
                Results.Ok(await pingService.RunScanAsync(GetActor(context), token)));
            ipAdminApi.MapPost("/{ipAddress}/ping", async (
                string ipAddress,
                HttpContext context,
                IpPingService pingService,
                CancellationToken token) =>
                Results.Ok(await pingService.RunSinglePingAsync(ipAddress, GetActor(context), token)));
            ipAdminApi.MapPut("/import", async (
                ImportIpInventoryRequest request,
                HttpContext context,
                RegistryDatabase database,
                CancellationToken token) =>
                Results.Ok(await database.ImportIpInventoryAsync(
                    request, GetActor(context), token)));
            ipAdminApi.MapGet("/{ipAddress}", async (
                string ipAddress,
                RegistryDatabase database,
                CancellationToken token) =>
                Results.Ok(await database.GetIpInventoryAsync(ipAddress, token)));
            ipAdminApi.MapPut("/{ipAddress}", async (
                string ipAddress,
                UpdateIpInventoryRequest request,
                HttpContext context,
                RegistryDatabase database,
                CancellationToken token) =>
                Results.Ok(await database.UpdateIpInventoryAsync(
                    ipAddress,
                    request,
                    GetActor(context),
                    token)));

            var ipRequestAdminApi = api.MapGroup("/ip-requests");
            ipRequestAdminApi.MapGet("", async (
                string? status,
                RegistryDatabase database,
                CancellationToken token) =>
                Results.Ok(await database.ListIpRequestsAsync(status, token)));
            ipRequestAdminApi.MapGet("/{requestId}", async (
                string requestId,
                RegistryDatabase database,
                CancellationToken token) =>
                Results.Ok(await database.GetIpRequestAsync(requestId, token)));
            ipRequestAdminApi.MapGet("/{requestId}/events", async (
                string requestId,
                RegistryDatabase database,
                CancellationToken token) =>
                Results.Ok(await database.ListIpRequestEventsAsync(requestId, token)));
            ipRequestAdminApi.MapPost("/{requestId}/approve", async (
                string requestId,
                ApproveIpRequestRequest request,
                HttpContext context,
                RegistryDatabase database,
                CancellationToken token) =>
                Results.Ok(await database.ApproveIpRequestAsync(
                    requestId, request, GetActor(context), token)));
            ipRequestAdminApi.MapPost("/{requestId}/reject", async (
                string requestId,
                RejectIpRequestRequest request,
                HttpContext context,
                RegistryDatabase database,
                CancellationToken token) =>
                Results.Ok(await database.RejectIpRequestAsync(
                    requestId, request, GetActor(context), token)));
        }

        if (featureOptions.GuestPortalEnabled)
        {
            var guestApi = api.MapGroup("/guest");
            guestApi.MapGet("/ip-addresses", async (
                RegistryDatabase database,
                CancellationToken token) =>
                Results.Ok(await database.ListGuestIpInventoryAsync(token)));
            guestApi.MapPost("/ip-requests", async (
                CreateIpRequestRequest request,
                RegistryDatabase database,
                CancellationToken token) =>
            {
                var created = await database.CreateIpRequestAsync(request, token);
                var receipt = new GuestIpRequestReceipt(
                    created.Id, created.IpAddress, created.Status, created.CreatedAt);
                return Results.Created($"/api/v1/guest/ip-requests/{created.Id}", receipt);
            }).RequireRateLimiting("guest-ip-requests");
        }

        api.MapGet("/groups", async (bool? includeDisabled, RegistryDatabase database, CancellationToken token) =>
            Results.Ok(await database.ListGroupsAsync(includeDisabled ?? false, token)));
        api.MapPost("/groups", async (
            CreateGroupRequest request,
            HttpContext context,
            RegistryDatabase database,
            CancellationToken token) =>
        {
            var group = await database.CreateGroupAsync(request, GetActor(context), token);
            return Results.Created($"/api/v1/groups/{group.Id}", group);
        });
        api.MapPut("/groups/{id:long}", async (
            long id,
            UpdateGroupRequest request,
            HttpContext context,
            RegistryDatabase database,
            CancellationToken token) =>
            Results.Ok(await database.UpdateGroupAsync(id, request, GetActor(context), token)));

        api.MapGet("/settings", async (RegistryDatabase database, CancellationToken token) =>
            Results.Ok(await database.ListSettingsAsync(token)));
        api.MapGet("/settings/{key}", async (string key, RegistryDatabase database, CancellationToken token) =>
            Results.Ok(await database.GetSettingAsync(key, token)));
        api.MapPut("/settings/{key}", async (
            string key,
            UpdateSettingRequest request,
            HttpContext context,
            RegistryDatabase database,
            CancellationToken token) =>
        {
            if (string.Equals(key, "ip-monitoring", StringComparison.OrdinalIgnoreCase))
            {
                var settings = request.Value.Deserialize<IpMonitoringSettings>(new JsonSerializerOptions(JsonSerializerDefaults.Web))
                    ?? throw new RegistryValidationException("IP monitoring settings are required.");
                RequestValidator.Normalize(settings);
            }

            return Results.Ok(await database.SaveSettingAsync(key, request, GetActor(context), token));
        });

        api.MapGet("/firmware-history", async (int? limit, RegistryDatabase database, CancellationToken token) =>
            Results.Ok(await database.ListFirmwareHistoryAsync(limit ?? 500, token)));
        api.MapPut("/firmware-history/{jobId}", async (
            string jobId,
            UpsertFirmwareHistoryRequest request,
            HttpContext context,
            RegistryDatabase database,
            CancellationToken token) =>
            Results.Ok(await database.UpsertFirmwareHistoryAsync(jobId, request, GetActor(context), token)));
        api.MapPut("/firmware-history/{jobId}/clone-file", async (
            string jobId,
            string fileName,
            HttpContext context,
            RegistryDatabase database,
            CancellationToken token) =>
            Results.Ok(await database.SaveFirmwareCloneFileAsync(
                jobId,
                fileName,
                context.Request.Body,
                context.Request.ContentLength,
                GetActor(context),
                token)));
        api.MapGet("/firmware-history/{jobId}/clone-file", async (
            string jobId,
            HttpContext context,
            RegistryDatabase database,
            CancellationToken token) =>
        {
            var file = await database.GetFirmwareCloneFileAsync(jobId, token);
            context.Response.Headers["X-QA-SHA256"] = file.Sha256;
            context.Response.Headers["X-QA-File-Name"] = file.FileName;
            return Results.File(file.FullPath, "application/xml", file.FileName, enableRangeProcessing: true);
        });
        api.MapDelete("/firmware-history/{jobId}", async (
            string jobId,
            HttpContext context,
            RegistryDatabase database,
            CancellationToken token) =>
        {
            await database.DeleteFirmwareHistoryAsync(jobId, GetActor(context), token);
            return Results.NoContent();
        });

        api.MapGet("/audit", async (int? limit, RegistryDatabase database, CancellationToken token) =>
            Results.Ok(await database.ListAuditAsync(limit ?? 100, token)));
        api.MapPost("/maintenance/backup", async (RegistryDatabase database, CancellationToken token) =>
            Results.Ok(await database.CreateBackupAsync(token)));

        return endpoints;
    }

    private static async Task<IResult> GetHealthAsync(RegistryDatabase database, CancellationToken token)
    {
        var status = await database.GetStatusAsync(token);
        var version = Assembly.GetExecutingAssembly().GetName().Version?.ToString(3) ?? "unknown";
        var healthy = string.Equals(status.IntegrityStatus, "ok", StringComparison.OrdinalIgnoreCase);
        var response = new HealthResponse(healthy ? "healthy" : "unhealthy", version, DateTimeOffset.UtcNow, status);
        return healthy ? Results.Ok(response) : Results.Json(response, statusCode: StatusCodes.Status503ServiceUnavailable);
    }

    private static string GetActor(HttpContext context)
    {
        var actor = context.Request.Headers["X-QA-User"].FirstOrDefault();
        return string.IsNullOrWhiteSpace(actor) ? "unknown" : actor;
    }
}
