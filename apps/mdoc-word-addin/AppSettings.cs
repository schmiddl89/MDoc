using System;
using System.IO;
using System.Text.RegularExpressions;

namespace MdocWordAddIn
{
    /// <summary>
    /// Reads %APPDATA%\MDoc\settings.json (shared with the desktop editor).
    /// Falls back to legacy ui-language / ribbon-placement files.
    /// </summary>
    internal static class AppSettings
    {
        private static bool _loaded;
        private static string _uiLanguage = "en";
        private static string _ribbonPlacement = "home";

        internal static string UiLanguage
        {
            get
            {
                EnsureLoaded();
                return _uiLanguage;
            }
        }

        internal static string RibbonPlacement
        {
            get
            {
                EnsureLoaded();
                return _ribbonPlacement;
            }
        }

        private static void EnsureLoaded()
        {
            if (_loaded)
            {
                return;
            }

            _loaded = true;
            _uiLanguage = "en";
            _ribbonPlacement = "home";

            try
            {
                var dir = InstallLayout.ResolveConfigDirectory();
                var settingsPath = Path.Combine(dir, "settings.json");
                if (File.Exists(settingsPath))
                {
                    var json = File.ReadAllText(settingsPath);
                    string lang;
                    if (TryReadJsonString(json, "uiLanguage", out lang))
                    {
                        _uiLanguage = NormalizeLanguage(lang);
                    }

                    string ribbon;
                    if (TryReadJsonString(json, "ribbonPlacement", out ribbon))
                    {
                        _ribbonPlacement = NormalizeRibbon(ribbon);
                    }

                    return;
                }

                _uiLanguage = NormalizeLanguage(ReadLegacyLine(Path.Combine(dir, "ui-language")));
                _ribbonPlacement = NormalizeRibbon(ReadLegacyLine(Path.Combine(dir, "ribbon-placement")));
            }
            catch
            {
                _uiLanguage = "en";
                _ribbonPlacement = "home";
            }
        }

        private static string ReadLegacyLine(string path)
        {
            if (!File.Exists(path))
            {
                return null;
            }

            using (var reader = new StringReader(File.ReadAllText(path)))
            {
                return (reader.ReadLine() ?? string.Empty).Trim();
            }
        }

        private static bool TryReadJsonString(string json, string propertyName, out string value)
        {
            value = null;
            if (string.IsNullOrEmpty(json) || string.IsNullOrEmpty(propertyName))
            {
                return false;
            }

            var pattern = "\"" + Regex.Escape(propertyName) + "\"\\s*:\\s*\"([^\"]+)\"";
            var match = Regex.Match(json, pattern, RegexOptions.IgnoreCase);
            if (!match.Success)
            {
                return false;
            }

            value = match.Groups[1].Value.Trim();
            return value.Length > 0;
        }

        private static string NormalizeLanguage(string raw)
        {
            if (string.IsNullOrWhiteSpace(raw))
            {
                return "en";
            }

            var token = raw.Trim().ToLowerInvariant();
            if (token == "de" || token.StartsWith("de-", StringComparison.Ordinal) || token.StartsWith("de_", StringComparison.Ordinal))
            {
                return "de";
            }

            return "en";
        }

        private static string NormalizeRibbon(string raw)
        {
            if (string.IsNullOrWhiteSpace(raw))
            {
                return "home";
            }

            var token = raw.Trim().ToLowerInvariant();
            if (token == "owntab" || token == "tab" || token == "mdoc" || token == "mdword")
            {
                return "ownTab";
            }

            return "home";
        }
    }
}
