// Prueba de humo del .saver: lo carga como lo haría macOS, anima 3 s y comprueba que dibuja.
//   swift .github/qa/smoke.swift "/Library/Screen Savers/ScreensaverFX.saver"
import AppKit
import ScreenSaver

var fails = 0
func check(_ name: String, _ ok: Bool) {
    print(ok ? "  OK    \(name)" : "  FALLA \(name)")
    if !ok { fails += 1 }
}

let path = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "/Library/Screen Savers/ScreensaverFX.saver"
_ = NSApplication.shared
guard let bundle = Bundle(path: path), bundle.load(), let cls = bundle.principalClass as? ScreenSaverView.Type else {
    print("  FALLA no se pudo cargar \(path)")
    exit(1)
}
check("bundle cargado (\(bundle.bundleIdentifier ?? "?"))", true)

/// Píxeles no negros de la imagen que muestra la vista.
func litPixels(_ view: NSView) -> Int {
    guard let c = view.layer?.sublayers?.first?.contents, CFGetTypeID(c as CFTypeRef) == CGImage.typeID else { return -1 }
    let img = c as! CGImage
    let (w, h) = (img.width, img.height)
    var buf = [UInt32](repeating: 0, count: w * h)
    let ctx = CGContext(data: &buf, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4,
                        space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.noneSkipFirst.rawValue)!
    ctx.draw(img, in: CGRect(x: 0, y: 0, width: w, height: h))
    return buf.reduce(0) { $0 + ($1 & 0x00FF_FFFF > 0x0010_1010 ? 1 : 0) }
}

for (preview, size) in [(false, NSSize(width: 1280, height: 720)), (true, NSSize(width: 296, height: 184)), (false, NSSize(width: 720, height: 1280))] {
    let label = preview ? "vista previa \(Int(size.width))x\(Int(size.height))" : "pantalla \(Int(size.width))x\(Int(size.height))"
    guard let view = cls.init(frame: NSRect(origin: .zero, size: size), isPreview: preview) else {
        check("\(label): se crea la vista", false)
        continue
    }
    let window = NSWindow(contentRect: view.frame, styleMask: .borderless, backing: .buffered, defer: false)
    window.contentView = view
    view.startAnimation()
    let t0 = Date()
    var frames = 0
    while Date().timeIntervalSince(t0) < 3 {
        view.animateOneFrame()
        frames += 1
        RunLoop.current.run(until: Date().addingTimeInterval(1.0 / 60.0))
    }
    view.stopAnimation()
    let lit = litPixels(view)
    check("\(label): \(frames) cuadros, \(lit) píxeles encendidos", lit > 100)
    if !preview {
        check("\(label): hoja de opciones", view.hasConfigureSheet && view.configureSheet != nil)
    }
}

var ru = rusage()
getrusage(RUSAGE_SELF, &ru)
print("  INFO  memoria máxima del proceso de prueba: \(ru.ru_maxrss / 1_048_576) MB")
print("Resultado: \(fails) fallos")
exit(Int32(fails))
