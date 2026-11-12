// Screensaver FX para macOS: ScreenSaverView que muestra el búfer de ssfx-core en una CALayer
// escalada con vecino más cercano (píxeles nítidos, sin trabajo extra de la CPU).

import AppKit
import QuartzCore
import ScreenSaver

let moduleID = "dev.thealejandro.ScreensaverFX"

enum Settings {
    static var defaults: ScreenSaverDefaults? { ScreenSaverDefaults(forModuleWithName: moduleID) }

    static func get(_ key: String, _ fallback: String) -> String {
        defaults?.string(forKey: key) ?? fallback
    }

    /// Las mismas claves que config.ini en Windows.
    static func ini() -> String {
        let one = { (s: String) in s.replacingOccurrences(of: "\n", with: " ") }
        return """
        banner=\(get("banner", "default"))
        title=\(one(get("title", "")))
        subtitle=\(one(get("subtitle", "")))
        theme=\(get("theme", "auto"))
        size=\(get("size", "m"))
        effects=\(get("effects", "all"))
        lang=\(get("lang", "auto"))
        screens=\(get("screens", "span"))
        font=\(get("font", "block"))
        calm=\(get("calm", "0"))
        """
    }

    static var art: String? { defaults?.string(forKey: "art") }
}

var systemIsSpanish: Bool { Locale.preferredLanguages.first?.hasPrefix("es") ?? false }

/// Semilla y reloj comunes a todas las pantallas. macOS crea una vista por pantalla (a veces en
/// procesos distintos); con la misma semilla y el mismo origen de tiempo cada motor calcula lo
/// mismo y los efectos pasan de una pantalla a otra sin hablar entre sí. CACurrentMediaTime es el
/// mismo reloj en todo el sistema.
enum Session {
    private static var seed: UInt64 = 0
    private static var start: Double = 0
    private static var lastSeen: Double = -.infinity
    private static var file: URL { URL(fileURLWithPath: NSTemporaryDirectory()).appendingPathComponent("ssfx-session") }

    /// La sesión en curso, o una nueva si ninguna pantalla la usó en los últimos segundos.
    static func current() -> (seed: UInt64, start: Double) {
        let now = CACurrentMediaTime()
        if now - lastSeen < 5 { return (seed, start) }
        // Otro proceso pudo empezarla hace un momento.
        if let s = try? String(contentsOf: file, encoding: .utf8) {
            let p = s.split(separator: " ")
            if p.count == 2, let sd = UInt64(p[0]), let st = Double(p[1]), now - st >= 0, now - st < 5 {
                (seed, start, lastSeen) = (sd, st, now)
                return (seed, start)
            }
        }
        (seed, start, lastSeen) = (UInt64.random(in: 0...UInt64.max), now, now)
        try? "\(seed) \(start)".write(to: file, atomically: true, encoding: .utf8)
        return (seed, start)
    }

    /// Mantiene viva la sesión mientras alguna pantalla anima.
    static func touch() { lastSeen = CACurrentMediaTime() }

    /// Todas las pantallas en puntos con y hacia abajo y su tamaño en píxeles; `me` es `mine`.
    static func desk(mine: NSScreen?, px: CGSize) -> (screens: [SsfxScreen], me: Int)? {
        let all = NSScreen.screens
        guard let mine = mine, let me = all.firstIndex(of: mine) else { return nil }
        let top = all.map { $0.frame.maxY }.max() ?? 0
        let screens = all.enumerated().map { i, s -> SsfxScreen in
            let f = s.frame, k = s.backingScaleFactor
            let (pw, ph) = i == me ? (px.width, px.height) : ((f.width * k).rounded(), (f.height * k).rounded())
            return SsfxScreen(x: f.minX, y: top - f.maxY, w: f.width, h: f.height, px_w: Int32(pw), px_h: Int32(ph))
        }
        return (screens, me)
    }
}

@objc(ScreensaverFXView)
final class ScreensaverFXView: ScreenSaverView {
    private var engine: OpaquePointer?
    private var start = CACurrentMediaTime()
    private let imageLayer = CALayer()
    private var engineSize = CGSize.zero
    private var sheet: ConfigSheet?

    override init?(frame: NSRect, isPreview: Bool) {
        super.init(frame: frame, isPreview: isPreview)
        animationTimeInterval = 1.0 / 60.0
        wantsLayer = true
        layer?.backgroundColor = NSColor.black.cgColor
        imageLayer.magnificationFilter = .nearest
        imageLayer.contentsGravity = .resize
        imageLayer.actions = ["contents": NSNull(), "bounds": NSNull(), "position": NSNull()]
        layer?.addSublayer(imageLayer)
        if !isPreview {
            // macOS 14+ no siempre detiene legacyScreenSaver al salir: cerramos nosotros.
            DistributedNotificationCenter.default().addObserver(forName: .init("com.apple.screensaver.willstop"), object: nil, queue: .main) { _ in
                exit(0)
            }
        }
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
    }

    deinit {
        if let e = engine { ssfx_free(e) }
    }

    private var scale: CGFloat { window?.backingScaleFactor ?? NSScreen.main?.backingScaleFactor ?? 2 }

    private func accent() -> UInt32 {
        guard let c = NSColor.controlAccentColor.usingColorSpace(.sRGB) else { return 0x0A84FF }
        let r = UInt32(c.redComponent * 255), g = UInt32(c.greenComponent * 255), b = UInt32(c.blueComponent * 255)
        return r << 16 | g << 8 | b
    }

    /// (Re)crea el motor con el tamaño actual en píxeles físicos.
    private func makeEngine() {
        let px = CGSize(width: (bounds.width * scale).rounded(), height: (bounds.height * scale).rounded())
        guard px.width > 0, px.height > 0, px != engineSize else { return }
        if let e = engine { ssfx_free(e) }
        let ini = Settings.ini()
        let banner = Settings.get("banner", "default")
        let art = banner == "file" ? Settings.art : nil
        // La vista previa va sola; en pantalla completa todas comparten escritorio, semilla y reloj.
        let desk = isPreview ? nil : Session.desk(mine: window?.screen, px: px)
        let session = isPreview ? (seed: UInt64.random(in: 0...UInt64.max), start: CACurrentMediaTime()) : Session.current()
        let (acc, es) = (accent(), Int32(systemIsSpanish ? 1 : 0))
        let make = { (iniPtr: UnsafePointer<CChar>, artPtr: UnsafePointer<CChar>?) -> OpaquePointer? in
            if let d = desk {
                return d.screens.withUnsafeBufferPointer { ssfx_new_desk($0.baseAddress, Int32(d.screens.count), Int32(d.me), iniPtr, artPtr, acc, session.seed, es) }
            }
            return ssfx_new(Int32(px.width), Int32(px.height), iniPtr, artPtr, acc, session.seed, es)
        }
        engine = ini.withCString { iniPtr in
            if let art = art { return art.withCString { make(iniPtr, $0) } }
            return make(iniPtr, nil)
        }
        engineSize = px
        start = session.start
        guard let e = engine else { return }
        var d = [Int32](repeating: 0, count: 4)
        let down = ssfx_dest(e, &d) != 0
        imageLayer.minificationFilter = down ? .trilinear : .nearest
        let s = scale
        // Capa en puntos, origen abajo a la izquierda.
        imageLayer.frame = CGRect(x: CGFloat(d[0]) / s, y: (px.height - CGFloat(d[1] + d[3])) / s, width: CGFloat(d[2]) / s, height: CGFloat(d[3]) / s)
    }

    private func now() -> SsfxWallTime {
        let c = Calendar.current.dateComponents([.year, .month, .day, .weekday, .hour, .minute, .second], from: Date())
        // Calendar: 1 = domingo.
        let wd = ((c.weekday ?? 2) + 5) % 7
        return SsfxWallTime(year: Int32(c.year ?? 2000), month: Int32(c.month ?? 1), day: Int32(c.day ?? 1), weekday: Int32(wd), hour: Int32(c.hour ?? 0), minute: Int32(c.minute ?? 0), second: Int32(c.second ?? 0))
    }

    private func present() {
        guard let e = engine else { return }
        var w: Int32 = 0, h: Int32 = 0
        guard let px = ssfx_pixels(e, &w, &h), w > 0, h > 0 else { return }
        let bytes = Int(w) * Int(h) * 4
        guard let data = CFDataCreate(nil, UnsafeRawPointer(px).assumingMemoryBound(to: UInt8.self), bytes),
              let provider = CGDataProvider(data: data) else { return }
        let info = CGBitmapInfo(rawValue: CGBitmapInfo.byteOrder32Little.rawValue | CGImageAlphaInfo.noneSkipFirst.rawValue)
        let img = CGImage(width: Int(w), height: Int(h), bitsPerComponent: 8, bitsPerPixel: 32, bytesPerRow: Int(w) * 4,
                          space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: info, provider: provider,
                          decode: nil, shouldInterpolate: false, intent: .defaultIntent)
        imageLayer.contents = img
        attachImageLayer()
    }

    /// La capa del view puede crearse o sustituirse al entrar en la ventana (después de init):
    /// asegura que la imagen cuelga de la capa actual.
    private func attachImageLayer() {
        guard let root = layer, imageLayer.superlayer !== root else { return }
        imageLayer.removeFromSuperlayer()
        root.backgroundColor = NSColor.black.cgColor
        root.addSublayer(imageLayer)
    }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        attachImageLayer()
        // Ya se sabe en qué pantalla está: rehace el motor para que conozca su sitio en el escritorio.
        if engine != nil && !isPreview && window != nil {
            engineSize = .zero
            makeEngine()
        }
    }

    override func startAnimation() {
        super.startAnimation()
        makeEngine()
    }

    override func setFrameSize(_ newSize: NSSize) {
        super.setFrameSize(newSize)
        if engine != nil { makeEngine() }
    }

    override func viewDidChangeBackingProperties() {
        super.viewDidChangeBackingProperties()
        layer?.contentsScale = scale
        if engine != nil { makeEngine() }
    }

    override func draw(_ rect: NSRect) {
        NSColor.black.setFill()
        rect.fill()
    }

    override func animateOneFrame() {
        if engine == nil { makeEngine() }
        guard let e = engine else { return }
        var t = now()
        if !isPreview { Session.touch() }
        if ssfx_tick(e, CACurrentMediaTime() - start, &t) != 0 {
            present()
        }
    }

    override var hasConfigureSheet: Bool { true }

    override var configureSheet: NSWindow? {
        let s = ConfigSheet { [weak self] in
            self?.engineSize = .zero
            self?.makeEngine()
        }
        sheet = s
        return s.window
    }
}
