using Microsoft.AspNetCore.Mvc;

namespace QAReceiverRegistry.Infrastructure;

public sealed class ApiExceptionMiddleware(RequestDelegate next, ILogger<ApiExceptionMiddleware> logger)
{
    public async Task InvokeAsync(HttpContext context)
    {
        try
        {
            await next(context);
        }
        catch (Exception exception)
        {
            if (context.Response.HasStarted)
            {
                throw;
            }

            var (status, title) = exception switch
            {
                RegistryValidationException => (StatusCodes.Status400BadRequest, "Invalid request"),
                EntityNotFoundException => (StatusCodes.Status404NotFound, "Not found"),
                DuplicateEntityException => (StatusCodes.Status409Conflict, "Duplicate record"),
                ConcurrencyConflictException => (StatusCodes.Status409Conflict, "Record changed"),
                _ => (StatusCodes.Status500InternalServerError, "Internal server error"),
            };

            if (status >= 500)
            {
                logger.LogError(exception, "Unhandled API error for {Method} {Path}", context.Request.Method, context.Request.Path);
            }
            else
            {
                logger.LogWarning(
                    "Rejected API request for {Method} {Path}: {Reason}",
                    context.Request.Method,
                    context.Request.Path,
                    exception.Message);
            }

            context.Response.Clear();
            context.Response.StatusCode = status;
            context.Response.ContentType = "application/problem+json";
            var problem = new ProblemDetails
            {
                Status = status,
                Title = title,
                Detail = status >= 500 ? "The request could not be completed." : exception.Message,
                Instance = context.Request.Path,
            };
            problem.Extensions["traceId"] = context.TraceIdentifier;
            await context.Response.WriteAsJsonAsync(problem, context.RequestAborted);
        }
    }
}
