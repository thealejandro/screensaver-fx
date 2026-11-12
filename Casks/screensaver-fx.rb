cask "screensaver-fx" do
  version "0.1.16"
  sha256 "c8f89dcf6a831608822abc31ba380f592be907f8ece14bbbe1aa44173599cc12"

  url "https://github.com/thealejandro/screensaver-fx/releases/download/v#{version}/ScreensaverFX-macOS.zip"
  name "Screensaver FX"
  desc "Native screensaver with the Omarchy animations and 41 terminal-style effects"
  homepage "https://github.com/thealejandro/screensaver-fx"

  livecheck do
    url :url
    strategy :github_latest
  end

  depends_on :macos

  screen_saver "ScreensaverFX.saver"

  # Sin firma de Apple: se quita la cuarentena antes de instalarlo para que macOS deje cargarlo.
  preflight_steps do
    run "/usr/bin/xattr", args: ["-dr", "com.apple.quarantine", "{{staged_path}}/ScreensaverFX.saver"]
  end

  zap trash: [
    "~/Library/Containers/com.apple.ScreenSaver.Engine.legacyScreenSaver/Data/Library/Preferences/ByHost/dev.thealejandro.ScreensaverFX.*",
    "~/Library/Preferences/ByHost/dev.thealejandro.ScreensaverFX.*",
  ]
end
