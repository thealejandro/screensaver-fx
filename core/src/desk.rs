//! Escritorio con varios monitores: dónde está cada pantalla y cómo se pasa de una celda de
//! cualquier pantalla a un punto común. Cada monitor tiene su propio motor, pero todos conocen el
//! escritorio entero y usan la misma semilla y el mismo reloj, así que calculan lo mismo y un
//! efecto puede salir por un borde y entrar por el monitor de al lado sin comunicarse entre sí.

use crate::raster::{CH, CW};
use crate::{compute_layout, Layout, Size};

/// Una pantalla en coordenadas del escritorio (píxeles en Windows, puntos en macOS) y su tamaño
/// en píxeles físicos.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Screen {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub px_w: i32,
    pub px_h: i32,
}

impl Screen {
    /// Pantalla sola en el origen, con las coordenadas del escritorio en píxeles.
    pub fn px(px_w: i32, px_h: i32) -> Screen {
        Screen { x: 0.0, y: 0.0, w: px_w as f64, h: px_h as f64, px_w, px_h }
    }

    fn valid(&self) -> bool {
        [self.x, self.y, self.w, self.h].iter().all(|v| v.is_finite()) && self.w > 0.0 && self.h > 0.0 && self.px_w > 0 && self.px_h > 0
    }
}

/// Una pantalla vista por el motor: su rejilla y dónde cae el rótulo.
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub screen: Screen,
    pub layout: Layout,
    /// Esquina del rótulo en la rejilla de esa pantalla.
    pub x0: i32,
    pub y0: i32,
    /// Unidades del escritorio por celda (ancho, alto).
    pub cell: (f32, f32),
    /// Origen de la celda 0,0 en el escritorio.
    origin: (f32, f32),
}

impl View {
    fn new(screen: Screen, layout: Layout, art_w: i32, art_h: i32) -> View {
        let k = layout.dst_w as f64 / layout.buf_w.max(1) as f64;
        let (ux, uy) = (screen.w / screen.px_w.max(1) as f64, screen.h / screen.px_h.max(1) as f64);
        View {
            screen,
            layout,
            x0: ((layout.cols - art_w) / 2).max(0),
            y0: ((layout.rows - art_h) / 2).max(0),
            cell: ((CW as f64 * k * ux).max(1e-3) as f32, (CH as f64 * k * uy).max(1e-3) as f32),
            origin: ((screen.x + layout.dst_x as f64 * ux) as f32, (screen.y + layout.dst_y as f64 * uy) as f32),
        }
    }

    /// Centro de la celda (cx, cy) en el escritorio.
    pub fn to_world(&self, cx: f32, cy: f32) -> (f32, f32) {
        (self.origin.0 + (cx + 0.5) * self.cell.0, self.origin.1 + (cy + 0.5) * self.cell.1)
    }

    /// Celda bajo un punto del escritorio (puede quedar fuera de la rejilla).
    pub fn to_cell(&self, wx: f32, wy: f32) -> (i32, i32) {
        (((wx - self.origin.0) / self.cell.0).floor() as i32, ((wy - self.origin.1) / self.cell.1).floor() as i32)
    }

    pub fn center(&self) -> (f32, f32) {
        ((self.screen.x + self.screen.w / 2.0) as f32, (self.screen.y + self.screen.h / 2.0) as f32)
    }

    /// Centro del rótulo en el escritorio.
    pub fn banner_center(&self, art_w: i32, art_h: i32) -> (f32, f32) {
        self.to_world(self.x0 as f32 + art_w as f32 / 2.0 - 0.5, self.y0 as f32 + art_h as f32 / 2.0 - 0.5)
    }
}

pub struct Desk {
    pub views: Vec<View>,
    /// Pantalla que dibuja este motor.
    pub me: usize,
    /// Pantallas de izquierda a derecha (y de arriba abajo si coinciden).
    pub order: Vec<usize>,
    /// Rectángulo que envuelve todas las pantallas: x, y, ancho, alto.
    pub bounds: (f32, f32, f32, f32),
    /// Tamaño medio de celda: la misma medida en todos los motores para velocidades y tamaños.
    pub unit: (f32, f32),
}

impl Desk {
    /// `screens`: todas las pantallas (las no válidas se ignoran); `me`: la de este motor.
    pub fn new(screens: &[Screen], me: usize, size: Size, min_cols: i32, min_rows: i32, art_w: i32, art_h: i32) -> Desk {
        let mine = screens.get(me).copied();
        let mut list: Vec<Screen> = screens.iter().copied().filter(Screen::valid).collect();
        let mut me = mine.and_then(|m| list.iter().position(|s| *s == m));
        if me.is_none() {
            // Sin pantalla propia válida: una sola con el tamaño pedido (o 1x1).
            let s = mine.filter(|m| m.px_w > 0 && m.px_h > 0).map(|m| Screen::px(m.px_w, m.px_h)).unwrap_or(Screen::px(1, 1));
            list = vec![s];
            me = Some(0);
        }
        let views: Vec<View> = list.iter().map(|&s| View::new(s, compute_layout(s.px_w, s.px_h, size, min_cols, min_rows), art_w, art_h)).collect();
        Desk::from_views(views, me.unwrap_or(0))
    }

    /// Una sola pantalla con la rejilla dada (para la vista previa y las pruebas).
    pub fn single(layout: Layout, art_w: i32, art_h: i32) -> Desk {
        let s = Screen::px(layout.dst_x * 2 + layout.dst_w.max(1), layout.dst_y * 2 + layout.dst_h.max(1));
        Desk::from_views(vec![View::new(s, layout, art_w, art_h)], 0)
    }

    fn from_views(views: Vec<View>, me: usize) -> Desk {
        let mut order: Vec<usize> = (0..views.len()).collect();
        order.sort_by(|&a, &b| {
            let (ca, cb) = (views[a].center(), views[b].center());
            ca.0.total_cmp(&cb.0).then(ca.1.total_cmp(&cb.1))
        });
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for v in &views {
            let s = v.screen;
            (x0, y0, x1, y1) = (x0.min(s.x), y0.min(s.y), x1.max(s.x + s.w), y1.max(s.y + s.h));
        }
        let n = views.len().max(1) as f32;
        let unit = views.iter().fold((0.0, 0.0), |a, v| (a.0 + v.cell.0 / n, a.1 + v.cell.1 / n));
        Desk { me, order, bounds: (x0 as f32, y0 as f32, (x1 - x0) as f32, (y1 - y0) as f32), unit, views }
    }

    pub fn mine(&self) -> &View {
        &self.views[self.me]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn three() -> Vec<Screen> {
        // 1440p a la izquierda, 1080p principal y 1080p vertical a la derecha, con alturas distintas.
        vec![
            Screen { x: 1920.0, y: -200.0, w: 1080.0, h: 1920.0, px_w: 1080, px_h: 1920 },
            Screen::px(1920, 1080),
            Screen { x: -2560.0, y: -360.0, w: 2560.0, h: 1440.0, px_w: 2560, px_h: 1440 },
        ]
    }

    #[test]
    fn order_and_bounds() {
        let d = Desk::new(&three(), 1, Size::M, 40, 20, 30, 10);
        assert_eq!(d.order, vec![2, 1, 0]);
        assert_eq!(d.bounds, (-2560.0, -360.0, 5560.0, 2080.0));
        assert_eq!(d.me, 1);
    }

    #[test]
    fn cells_round_trip_through_the_desk() {
        for me in 0..3 {
            let d = Desk::new(&three(), me, Size::M, 40, 20, 30, 10);
            for v in &d.views {
                for (cx, cy) in [(0, 0), (v.layout.cols - 1, v.layout.rows - 1), (v.x0, v.y0), (7, 3)] {
                    let (wx, wy) = v.to_world(cx as f32, cy as f32);
                    assert_eq!(v.to_cell(wx, wy), (cx, cy));
                    let s = v.screen;
                    assert!(wx >= s.x as f32 && wx <= (s.x + s.w) as f32 && wy >= s.y as f32 && wy <= (s.y + s.h) as f32);
                }
            }
        }
    }

    #[test]
    fn every_engine_sees_the_same_desk() {
        let a = Desk::new(&three(), 0, Size::M, 40, 20, 30, 10);
        let b = Desk::new(&three(), 2, Size::M, 40, 20, 30, 10);
        assert_eq!((a.order.clone(), a.bounds, a.unit), (b.order.clone(), b.bounds, b.unit));
        for (va, vb) in a.views.iter().zip(&b.views) {
            assert_eq!(va.to_world(3.0, 4.0), vb.to_world(3.0, 4.0));
        }
    }

    #[test]
    fn retina_points_and_invalid_screens() {
        // macOS: 1512x982 puntos a 2x junto a un externo 1x.
        let s = [Screen { x: 0.0, y: 0.0, w: 1512.0, h: 982.0, px_w: 3024, px_h: 1964 }, Screen { x: 1512.0, y: 0.0, w: 1920.0, h: 1080.0, px_w: 1920, px_h: 1080 }];
        let d = Desk::new(&s, 0, Size::M, 40, 20, 30, 10);
        let (a, b) = (d.views[0].cell, d.views[1].cell);
        assert!(a.0 > 0.0 && b.0 > 0.0);
        // Las celdas miden en el escritorio parecido aunque una pantalla tenga el doble de píxeles.
        assert!((a.0 / b.0 - 1.0).abs() < 0.5, "{a:?} {b:?}");
        let bad = [Screen { w: f64::NAN, ..s[0] }, Screen { px_w: 0, ..s[1] }];
        let d = Desk::new(&bad, 1, Size::M, 40, 20, 30, 10);
        assert_eq!(d.views.len(), 1);
        let d = Desk::new(&[], 3, Size::M, 40, 20, 30, 10);
        assert_eq!((d.views.len(), d.me), (1, 0));
    }
}
