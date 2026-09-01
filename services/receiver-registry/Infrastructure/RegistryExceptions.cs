namespace QAReceiverRegistry.Infrastructure;

public sealed class EntityNotFoundException(string message) : Exception(message);

public sealed class ConcurrencyConflictException(string message) : Exception(message);

public sealed class RegistryValidationException(string message) : Exception(message);

public sealed class DuplicateEntityException(string message) : Exception(message);
