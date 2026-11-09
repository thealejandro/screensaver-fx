// Hoja de opciones de Screensaver FX (Ajustes del Sistema > Salvapantallas > Opciones…).

import AppKit
import ScreenSaver

private func tr(_ es: String, _ en: String) -> String { systemIsSpanish ? es : en }

private func cstr(_ p: UnsafePointer<CChar>?) -> String { p.map { String(cString: $0) } ?? "" }

final class ConfigSheet: NSObject {
    let window: NSWindow
    private let onSave: () -> Void

    private let banner = NSPopUpButton()
    private let title = NSTextField()
    private let subtitle = NSTextField()
    private let artView = NSTextView()
    private let artScroll = NSScrollView()
    private let theme = NSPopUpButton()
    private let size = NSPopUpButton()
    private let effects = NSPopUpButton()
    private let lang = NSPopUpButton()

    private let banners = ["default", "text", "file"]
    private let sizes = ["s", "m", "l"]
    private let langs = ["auto", "es", "en"]
    private var themeIDs: [String] = ["auto", "random"]
    private var effectIDs: [String] = ["all", "classic", "modern"]

    init(onSave: @escaping () -> Void) {
        self.onSave = onSave
        window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 480, height: 470), styleMask: [.titled], backing: .buffered, defer: true)
        super.init()
        build()
        load()
    }

    private func row(_ label: String, _ control: NSView) -> NSView {
        let l = NSTextField(labelWithString: label)
        l.alignment = .right
        l.widthAnchor.constraint(equalToConstant: 130).isActive = true
        control.widthAnchor.constraint(equalToConstant: 290).isActive = true
        let s = NSStackView(views: [l, control])
        s.spacing = 10
        s.alignment = .firstBaseline
        return s
    }

    private func fill(_ p: NSPopUpButton, _ items: [String]) {
        p.removeAllItems()
        p.addItems(withTitles: items)
    }

    private func build() {
        fill(banner, [tr("JACC · XACODE (por defecto)", "JACC · XACODE (default)"), tr("Texto personalizado", "Custom text"), tr("Arte ASCII propio", "Own ASCII art")])
        banner.target = self
        banner.action = #selector(syncEnabled)
        title.placeholderString = tr("Título (hasta 16 caracteres)", "Title (up to 16 characters)")
        subtitle.placeholderString = tr("Subtítulo (opcional)", "Subtitle (optional)")

        artView.font = .monospacedSystemFont(ofSize: 10, weight: .regular)
        artView.isRichText = false
        artView.isAutomaticQuoteSubstitutionEnabled = false
        artView.isAutomaticDashSubstitutionEnabled = false
        artScroll.documentView = artView
        artScroll.hasVerticalScroller = true
        artScroll.hasHorizontalScroller = true
        artScroll.borderType = .bezelBorder
        artView.isHorizontallyResizable = true
        artView.textContainer?.widthTracksTextView = false
        artView.textContainer?.containerSize = NSSize(width: 4000, height: CGFloat.greatestFiniteMagnitude)
        artScroll.heightAnchor.constraint(equalToConstant: 110).isActive = true

        var themeNames = [tr("Color de acento del sistema", "System accent color"), tr("Aleatorio (cambia en cada efecto)", "Random (changes every effect)")]
        for i in 0..<ssfx_theme_count() {
            themeIDs.append(cstr(ssfx_theme_id(i)))
            themeNames.append(cstr(ssfx_theme_name(i)))
        }
        fill(theme, themeNames)
        fill(size, [tr("Pequeño", "Small"), tr("Mediano", "Medium"), tr("Grande", "Large")])
        var fxNames = [tr("Todos", "All"), tr("Clásicos de Omarchy", "Omarchy classics"), tr("Solo los nuevos", "New ones only")]
        for i in 0..<ssfx_fx_count() {
            effectIDs.append(cstr(ssfx_fx_id(i)))
            fxNames.append(tr("Solo: ", "Only: ") + cstr(ssfx_fx_name(i, systemIsSpanish ? 1 : 0)))
        }
        fill(effects, fxNames)
        fill(lang, [tr("Del sistema", "System"), "Español", "English"])

        let cancel = NSButton(title: tr("Cancelar", "Cancel"), target: self, action: #selector(close))
        cancel.keyEquivalent = "\u{1b}"
        let save = NSButton(title: tr("Guardar", "Save"), target: self, action: #selector(saveAndClose))
        save.keyEquivalent = "\r"
        let buttons = NSStackView(views: [cancel, save])
        buttons.spacing = 10

        let stack = NSStackView(views: [
            row(tr("Rótulo", "Banner"), banner),
            row(tr("Título", "Title"), title),
            row(tr("Subtítulo", "Subtitle"), subtitle),
            row(tr("Arte ASCII", "ASCII art"), artScroll),
            row(tr("Colores", "Colors"), theme),
            row(tr("Tamaño", "Size"), size),
            row(tr("Efectos", "Effects"), effects),
            row(tr("Idioma del reloj", "Clock language"), lang),
            buttons,
        ])
        stack.orientation = .vertical
        stack.alignment = .trailing
        stack.spacing = 10
        stack.edgeInsets = NSEdgeInsets(top: 20, left: 20, bottom: 20, right: 20)
        stack.translatesAutoresizingMaskIntoConstraints = false
        let content = NSView()
        content.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.leadingAnchor.constraint(equalTo: content.leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: content.trailingAnchor),
            stack.topAnchor.constraint(equalTo: content.topAnchor),
            stack.bottomAnchor.constraint(equalTo: content.bottomAnchor),
        ])
        window.contentView = content
    }

    private func select(_ p: NSPopUpButton, _ ids: [String], _ key: String, _ fallback: String) {
        p.selectItem(at: ids.firstIndex(of: Settings.get(key, fallback)) ?? ids.firstIndex(of: fallback) ?? 0)
    }

    private func load() {
        select(banner, banners, "banner", "default")
        title.stringValue = Settings.get("title", "")
        subtitle.stringValue = Settings.get("subtitle", "")
        artView.string = Settings.art ?? ""
        select(theme, themeIDs, "theme", "auto")
        select(size, sizes, "size", "m")
        select(effects, effectIDs, "effects", "all")
        select(lang, langs, "lang", "auto")
        syncEnabled()
    }

    @objc private func syncEnabled() {
        let b = banner.indexOfSelectedItem
        title.isEnabled = b == 1
        subtitle.isEnabled = b == 1
        artView.isEditable = b == 2
        artView.textColor = b == 2 ? .labelColor : .disabledControlTextColor
    }

    @objc private func saveAndClose() {
        guard let d = Settings.defaults else { return close() }
        d.set(banners[max(0, banner.indexOfSelectedItem)], forKey: "banner")
        d.set(String(title.stringValue.prefix(16)), forKey: "title")
        d.set(String(subtitle.stringValue.prefix(32)), forKey: "subtitle")
        d.set(artView.string, forKey: "art")
        d.set(themeIDs[max(0, theme.indexOfSelectedItem)], forKey: "theme")
        d.set(sizes[max(0, size.indexOfSelectedItem)], forKey: "size")
        d.set(effectIDs[max(0, effects.indexOfSelectedItem)], forKey: "effects")
        d.set(langs[max(0, lang.indexOfSelectedItem)], forKey: "lang")
        d.synchronize()
        onSave()
        close()
    }

    @objc private func close() {
        if let parent = window.sheetParent {
            parent.endSheet(window)
        } else {
            window.close()
        }
    }
}
