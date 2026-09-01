using System.Security.Cryptography.X509Certificates;

namespace QAReceiverRegistry.Infrastructure;

public static class CertificateLoader
{
    public static X509Certificate2 LoadFromLocalMachine(string thumbprint)
    {
        var normalized = NormalizeThumbprint(thumbprint);
        using var store = new X509Store(StoreName.My, StoreLocation.LocalMachine);
        store.Open(OpenFlags.ReadOnly | OpenFlags.OpenExistingOnly);

        var certificate = store.Certificates
            .Find(X509FindType.FindByThumbprint, normalized, validOnly: true)
            .OfType<X509Certificate2>()
            .FirstOrDefault();

        if (certificate is null)
        {
            throw new InvalidOperationException(
                $"A valid LocalMachine/My certificate with thumbprint {normalized} was not found.");
        }

        if (!certificate.HasPrivateKey)
        {
            certificate.Dispose();
            throw new InvalidOperationException($"Certificate {normalized} does not have a private key.");
        }

        return certificate;
    }

    private static string NormalizeThumbprint(string value)
    {
        return new string((value ?? string.Empty).Where(Uri.IsHexDigit).ToArray()).ToUpperInvariant();
    }
}
