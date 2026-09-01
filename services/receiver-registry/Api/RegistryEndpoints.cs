using System.Reflection;
using QAReceiverRegistry.Data;
using QAReceiverRegistry.Models;

namespace QAReceiverRegistry.Api;

public static class RegistryEndpoints
{
    public static IEndpointRouteBuilder MapRegistryEndpoints(this IEndpointRouteBuilder endpoints)
    {
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
            Results.Ok(await database.SaveSettingAsync(key, request, GetActor(context), token)));

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
