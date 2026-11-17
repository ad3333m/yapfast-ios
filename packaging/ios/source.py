"""Writes the KravaSign source (AltStore's format, as ad3333m's other feeds
have it) for one build: python3 source.py OWNER/REPO TAG BUILD SIZE."""
import datetime, json, sys

repo, tag, build, size = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
raw = f"https://raw.githubusercontent.com/{repo}"
icon = f"{raw}/ios/packaging/ios/AppIcon-1024.png"
app = {
    "name": "Discord (Yapfast)",
    "bundleIdentifier": "io.github.ad3333m.yapfast",
    "developerName": "ad3333m",
    "subtitle": "Discord, native in Rust",
    "version": "0.4.1",
    "buildVersion": build,
    "versionDate": datetime.date.today().isoformat(),
    "downloadURL": f"https://github.com/{repo}/releases/download/{tag}/Yapfast.ipa",
    "localizedDescription": "A native Discord client written in Rust: plugins in the manner of Vencord's, Endcord themes, Switch Accounts and EnhancedDecor. On iPhone, sharing your screen, playing video files and watching others' streams aren't there yet.",
    "iconURL": icon,
    "tintColor": "5865F2",
    "size": size,
}
print(json.dumps({
    "name": "Discord (Yapfast)",
    "identifier": "io.github.ad3333m.yapfast.source",
    "subtitle": "Builds of Discord (Yapfast) for iPhone",
    "iconURL": icon,
    "website": f"https://github.com/{repo}",
    "sourceURL": f"{raw}/feed/repo.json",
    "tintColor": "5865F2",
    "featuredApps": [app["bundleIdentifier"]],
    "apps": [app],
    "news": [],
}, indent=2))
