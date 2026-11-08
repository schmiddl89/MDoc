using System;
using System.IO;
using System.Reflection;

namespace MdocWordAddIn
{
    /// <summary>
    /// Installations- und Config-Pfade finden. Wichtig: Bei COM-Add-ins ist
    /// <see cref="AppDomain.BaseDirectory"/> oft der Word-Ordner, nicht der Add-in-Ordner.
    /// Deshalb immer über die Assembly-Location auflösen.
    /// </summary>
    internal static class InstallLayout
    {
        private static readonly string[] InstallFolderNames = new string[] { "MDoc", "mdoc", "mdword" };

        /// <summary>
        /// Ordner mit mdoc-cli.exe / mdoc-desktop.exe (z. B. %LOCALAPPDATA%\MDoc).
        /// </summary>
        internal static string ResolveInstallDirectory()
        {
            string assemblyPath = null;
            try
            {
                assemblyPath = Assembly.GetExecutingAssembly().Location;
            }
            catch
            {
            }

            if (!string.IsNullOrEmpty(assemblyPath))
            {
                string addinDir = Path.GetDirectoryName(assemblyPath);
                if (!string.IsNullOrEmpty(addinDir))
                {
                    string parent = Path.GetDirectoryName(addinDir);
                    if (IsInstallDirectory(parent))
                    {
                        return parent;
                    }

                    if (IsInstallDirectory(addinDir))
                    {
                        return addinDir;
                    }
                }
            }

            string local = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
            for (int i = 0; i < InstallFolderNames.Length; i++)
            {
                string candidate = Path.Combine(local, InstallFolderNames[i]);
                if (IsInstallDirectory(candidate))
                {
                    return candidate;
                }
            }

            return null;
        }

        /// <summary>
        /// %APPDATA%\MDoc — migriert einmalig von mdoc/mdword und korrigiert die Schreibweise.
        /// </summary>
        internal static string ResolveConfigDirectory()
        {
            string appData = Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData);
            const string preferredName = "MDoc";
            string preferred = Path.Combine(appData, preferredName);

            NormalizeConfigDirCasing(appData, preferredName);

            if (Directory.Exists(preferred))
            {
                return preferred;
            }

            foreach (string legacyName in new string[] { "mdword" })
            {
                string legacy = Path.Combine(appData, legacyName);
                if (!Directory.Exists(legacy))
                {
                    continue;
                }
                try
                {
                    Directory.Move(legacy, preferred);
                    return preferred;
                }
                catch
                {
                    return legacy;
                }
            }

            return preferred;
        }

        /// <summary>
        /// Windows: `mdoc` und `MDoc` sind oft derselbe Ordner — Schreibweise auf MDoc bringen.
        /// </summary>
        private static void NormalizeConfigDirCasing(string appData, string preferredName)
        {
            try
            {
                if (!Directory.Exists(appData))
                {
                    return;
                }
                foreach (string path in Directory.GetDirectories(appData))
                {
                    string name = Path.GetFileName(path);
                    if (!string.Equals(name, preferredName, StringComparison.OrdinalIgnoreCase)
                        || string.Equals(name, preferredName, StringComparison.Ordinal))
                    {
                        continue;
                    }
                    string temp = Path.Combine(appData, preferredName + ".__casing__");
                    string preferred = Path.Combine(appData, preferredName);
                    Directory.Move(path, temp);
                    Directory.Move(temp, preferred);
                    return;
                }
            }
            catch
            {
                /* optional — nächster Start versucht erneut */
            }
        }

        internal static string ResolveCliExe()
        {
            return ResolveFile("mdoc-cli.exe", "mdword-cli.exe");
        }

        internal static string ResolveDesktopExe()
        {
            return ResolveFile("mdoc-desktop.exe", "mdword-desktop.exe");
        }

        private static string ResolveFile(params string[] fileNames)
        {
            string installDir = ResolveInstallDirectory();
            if (installDir != null)
            {
                for (int i = 0; i < fileNames.Length; i++)
                {
                    string direct = Path.Combine(installDir, fileNames[i]);
                    if (File.Exists(direct))
                    {
                        return direct;
                    }
                }
            }

            // Fallbacks: LocalAppData\MDoc|mdoc|mdword und neben der DLL
            string local = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
            for (int i = 0; i < InstallFolderNames.Length; i++)
            {
                for (int j = 0; j < fileNames.Length; j++)
                {
                    string candidate = Path.Combine(local, InstallFolderNames[i], fileNames[j]);
                    if (File.Exists(candidate))
                    {
                        return candidate;
                    }
                }
            }

            try
            {
                string assemblyPath = Assembly.GetExecutingAssembly().Location;
                if (!string.IsNullOrEmpty(assemblyPath))
                {
                    string addinDir = Path.GetDirectoryName(assemblyPath);
                    for (int j = 0; j < fileNames.Length; j++)
                    {
                        string beside = Path.Combine(addinDir ?? string.Empty, fileNames[j]);
                        if (File.Exists(beside))
                        {
                            return beside;
                        }

                        string parent = Path.GetDirectoryName(addinDir);
                        string up = Path.Combine(parent ?? string.Empty, fileNames[j]);
                        if (File.Exists(up))
                        {
                            return up;
                        }
                    }
                }
            }
            catch
            {
            }

            return null;
        }

        private static bool IsInstallDirectory(string path)
        {
            if (string.IsNullOrEmpty(path) || !Directory.Exists(path))
            {
                return false;
            }

            return File.Exists(Path.Combine(path, "mdoc-cli.exe"))
                || File.Exists(Path.Combine(path, "mdoc-desktop.exe"))
                || File.Exists(Path.Combine(path, "mdword-cli.exe"))
                || File.Exists(Path.Combine(path, "mdword-desktop.exe"));
        }
    }
}
