use std::{
    env,
    fs::File,
    io::{self, BufWriter, Write},
    path::Path,
    time::Instant,
};

use minifb::{Key, KeyRepeat, Window, WindowOptions};

const EPS: f32 = 0.001;
const MAX_DEPTH: u32 = 3;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct V {
    x: f32,
    y: f32,
    z: f32,
}
impl V {
    const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
    fn dot(self, b: Self) -> f32 {
        self.x * b.x + self.y * b.y + self.z * b.z
    }
    fn cross(self, b: Self) -> Self {
        Self::new(
            self.y * b.z - self.z * b.y,
            self.z * b.x - self.x * b.z,
            self.x * b.y - self.y * b.x,
        )
    }
    fn len(self) -> f32 {
        self.dot(self).sqrt()
    }
    fn norm(self) -> Self {
        let l = self.len();
        if l < EPS {
            self
        } else {
            self / l
        }
    }
    fn clamp(self) -> Self {
        Self::new(
            self.x.clamp(0.0, 1.0),
            self.y.clamp(0.0, 1.0),
            self.z.clamp(0.0, 1.0),
        )
    }
    fn mul(self, b: Self) -> Self {
        Self::new(self.x * b.x, self.y * b.y, self.z * b.z)
    }
    fn lerp(self, b: Self, t: f32) -> Self {
        self * (1.0 - t) + b * t
    }
}
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub};
impl Add for V {
    type Output = Self;
    fn add(self, b: Self) -> Self {
        Self::new(self.x + b.x, self.y + b.y, self.z + b.z)
    }
}
impl AddAssign for V {
    fn add_assign(&mut self, b: Self) {
        *self = *self + b;
    }
}
impl Sub for V {
    type Output = Self;
    fn sub(self, b: Self) -> Self {
        Self::new(self.x - b.x, self.y - b.y, self.z - b.z)
    }
}
impl Mul<f32> for V {
    type Output = Self;
    fn mul(self, b: f32) -> Self {
        Self::new(self.x * b, self.y * b, self.z * b)
    }
}
impl Div<f32> for V {
    type Output = Self;
    fn div(self, b: f32) -> Self {
        Self::new(self.x / b, self.y / b, self.z / b)
    }
}
impl Neg for V {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

#[derive(Clone, Copy)]
struct Ray {
    o: V,
    d: V,
}
#[derive(Clone, Copy)]
struct Material {
    kind: Kind,
    albedo: V,
    specular: f32,
    reflect: f32,
    transparency: f32,
    ior: f32,
    emission: V,
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Grass,
    Brick,
    Pipe,
    Metal,
    Water,
    Star,
    Stone,
}
impl Material {
    fn new(
        kind: Kind,
        albedo: V,
        specular: f32,
        reflect: f32,
        transparency: f32,
        ior: f32,
        emission: V,
    ) -> Self {
        Self {
            kind,
            albedo,
            specular,
            reflect,
            transparency,
            ior,
            emission,
        }
    }
}
fn mat(k: Kind) -> Material {
    match k {
        Kind::Grass => Material::new(
            k,
            V::new(0.15, 0.48, 0.12),
            0.12,
            0.02,
            0.,
            1.,
            V::default(),
        ),
        Kind::Brick => Material::new(
            k,
            V::new(0.62, 0.12, 0.06),
            0.18,
            0.04,
            0.,
            1.,
            V::default(),
        ),
        Kind::Pipe => Material::new(
            k,
            V::new(0.03, 0.58, 0.14),
            0.35,
            0.12,
            0.,
            1.,
            V::default(),
        ),
        Kind::Metal => Material::new(k, V::new(0.72, 0.76, 0.82), 0.9, 0.72, 0., 1., V::default()),
        Kind::Water => Material::new(
            k,
            V::new(0.08, 0.35, 0.52),
            0.85,
            0.16,
            0.62,
            1.33,
            V::default(),
        ),
        Kind::Star => Material::new(
            k,
            V::new(1.0, 0.52, 0.04),
            0.5,
            0.08,
            0.,
            1.,
            V::new(1.4, 0.42, 0.03),
        ),
        Kind::Stone => Material::new(k, V::new(0.32, 0.35, 0.4), 0.15, 0.08, 0., 1., V::default()),
    }
}

#[derive(Clone, Copy)]
struct Cube {
    min: V,
    max: V,
    material: Material,
}
struct Hit {
    t: f32,
    point: V,
    normal: V,
    material: Material,
    u: f32,
    v: f32,
}
fn axis_t(o: f32, d: f32, min: f32, max: f32) -> (f32, f32) {
    if d.abs() < 1e-7 {
        if o < min || o > max {
            (f32::INFINITY, f32::NEG_INFINITY)
        } else {
            (f32::NEG_INFINITY, f32::INFINITY)
        }
    } else {
        let a = (min - o) / d;
        let b = (max - o) / d;
        (a.min(b), a.max(b))
    }
}
fn hit_cube(c: Cube, r: Ray) -> Option<Hit> {
    let (x0, x1) = axis_t(r.o.x, r.d.x, c.min.x, c.max.x);
    let (y0, y1) = axis_t(r.o.y, r.d.y, c.min.y, c.max.y);
    let (z0, z1) = axis_t(r.o.z, r.d.z, c.min.z, c.max.z);
    let enter = x0.max(y0).max(z0);
    let exit = x1.min(y1).min(z1);
    if exit < enter || exit < EPS {
        return None;
    }
    let t = if enter > EPS { enter } else { exit };
    let p = r.o + r.d * t;
    let dx = (p.x - c.min.x).min(c.max.x - p.x);
    let dy = (p.y - c.min.y).min(c.max.y - p.y);
    let dz = (p.z - c.min.z).min(c.max.z - p.z);
    let (n, u, v) = if dx <= dy && dx <= dz {
        let n = if (p.x - c.min.x) < (c.max.x - p.x) {
            V::new(-1., 0., 0.)
        } else {
            V::new(1., 0., 0.)
        };
        (
            n,
            (p.z - c.min.z) / (c.max.z - c.min.z),
            (p.y - c.min.y) / (c.max.y - c.min.y),
        )
    } else if dy <= dz {
        let n = if (p.y - c.min.y) < (c.max.y - p.y) {
            V::new(0., -1., 0.)
        } else {
            V::new(0., 1., 0.)
        };
        (
            n,
            (p.x - c.min.x) / (c.max.x - c.min.x),
            (p.z - c.min.z) / (c.max.z - c.min.z),
        )
    } else {
        let n = if (p.z - c.min.z) < (c.max.z - p.z) {
            V::new(0., 0., -1.)
        } else {
            V::new(0., 0., 1.)
        };
        (
            n,
            (p.x - c.min.x) / (c.max.x - c.min.x),
            (p.y - c.min.y) / (c.max.y - c.min.y),
        )
    };
    Some(Hit {
        t,
        point: p,
        normal: n,
        material: c.material,
        u,
        v,
    })
}
fn ry(v: V, a: f32) -> V {
    let (s, c) = a.sin_cos();
    V::new(c * v.x + s * v.z, v.y, -s * v.x + c * v.z)
}
fn texture(m: Material, u: f32, v: f32) -> V {
    let (x, y) = ((u * 8.).floor() as i32, (v * 8.).floor() as i32);
    match m.kind {
        Kind::Brick => {
            if (x + y) % 2 == 0 {
                m.albedo * 1.15
            } else {
                m.albedo * 0.72
            }
        }
        Kind::Grass => {
            let n = ((u * 37.0).sin() * (v * 91.0).cos()).abs();
            m.albedo * (0.78 + n * 0.35)
        }
        Kind::Pipe => {
            let stripe = ((u * 12.).sin().abs() * 0.12) + 0.9;
            m.albedo * stripe
        }
        Kind::Metal => {
            if (x + y) % 2 == 0 {
                m.albedo * 1.05
            } else {
                m.albedo * 0.88
            }
        }
        Kind::Water => m.albedo * (0.85 + 0.15 * (u * 20.).sin().abs()),
        Kind::Star => m.albedo * (0.9 + 0.1 * (u * 30.).sin().abs()),
        Kind::Stone => {
            if (x + y) % 3 == 0 {
                m.albedo * 1.3
            } else {
                m.albedo * 0.8
            }
        }
    }
    .clamp()
}

#[derive(Clone)]
struct Scene {
    name: &'static str,
    cubes: Vec<Cube>,
    lights: Vec<(V, V)>,
    yaw: f32,
    sky: u8,
}
fn cube(out: &mut Vec<Cube>, center: V, size: V, k: Kind) {
    let h = size * 0.5;
    out.push(Cube {
        min: center - h,
        max: center + h,
        material: mat(k),
    });
}
fn scene(id: usize) -> Scene {
    let mut c = Vec::new();
    match id % 3 {
        0 => {
            for x in -6..=6 {
                for z in -5..=5 {
                    cube(
                        &mut c,
                        V::new(x as f32 - 0.5, 0., z as f32),
                        V::new(1., 0.5, 1.),
                        Kind::Stone,
                    );
                }
            }
            cube(
                &mut c,
                V::new(0., 0.5, 0.),
                V::new(12., 0.5, 10.),
                Kind::Water,
            );
            for x in -4..=4 {
                cube(
                    &mut c,
                    V::new(x as f32, 1.2, -2.),
                    V::new(1., 1.4, 0.8),
                    Kind::Brick,
                );
            }
            cube(
                &mut c,
                V::new(0., 2.4, -2.),
                V::new(2.5, 1., 0.8),
                Kind::Metal,
            );
            for x in -3..=3 {
                cube(
                    &mut c,
                    V::new(x as f32, 1., 2.),
                    V::new(0.8, 2., 0.8),
                    Kind::Metal,
                );
            }
        }
        1 => {
            for x in -6..=6 {
                for z in -5..=5 {
                    cube(
                        &mut c,
                        V::new(x as f32 - 0.5, 0., z as f32),
                        V::new(1., 0.5, 1.),
                        Kind::Stone,
                    );
                }
            }
            for (p, s, k) in [
                (V::new(-3., 1., 0.), V::new(2., 2., 2.), Kind::Stone),
                (V::new(3., 1.5, -1.), V::new(3., 3., 3.), Kind::Grass),
                (V::new(0., 2., 2.), V::new(1., 1., 1.), Kind::Star),
            ] {
                cube(&mut c, p, s, k);
            }
            for i in 0..10 {
                let a = i as f32 * 0.628;
                cube(
                    &mut c,
                    V::new(a.cos() * 4., 1.2, a.sin() * 4.),
                    V::new(0.45, 0.45, 0.45),
                    Kind::Star,
                );
            }
        }
        _ => {
            for x in -6..=6 {
                for z in -5..=5 {
                    cube(
                        &mut c,
                        V::new(x as f32 - 0.5, 0., z as f32),
                        V::new(1., 0.5, 1.),
                        Kind::Grass,
                    );
                }
            }
            for x in -4..=4 {
                for y in 0..3 {
                    cube(
                        &mut c,
                        V::new(x as f32, y as f32 + 0.5, -2.),
                        V::new(1., 1., 0.8),
                        if y == 1 { Kind::Brick } else { Kind::Stone },
                    );
                }
            }
            for x in [-4., 4.] {
                cube(&mut c, V::new(x, 1.5, 2.), V::new(1., 3., 1.), Kind::Pipe);
                cube(&mut c, V::new(x, 3., 2.), V::new(2., 1., 2.), Kind::Pipe);
            }
            cube(&mut c, V::new(0., 1., 2.), V::new(2., 2., 1.), Kind::Brick);
        }
    }
    let name = match id % 3 {
        0 => "Mario Odyssey",
        1 => "Mario Galaxy",
        _ => "NSMB Wii",
    };
    let lights = match id % 3 {
        0 => vec![(V::new(-4., 7., -4.), V::new(1., 0.78, 0.55))],
        1 => vec![
            (V::new(0., 7., 0.), V::new(0.7, 0.8, 1.)),
            (V::new(-4., 3., 3.), V::new(1., 0.3, 0.1)),
        ],
        _ => vec![(V::new(-3., 7., -4.), V::new(1., 0.9, 0.65))],
    };
    Scene {
        name,
        cubes: c,
        lights,
        yaw: 0.,
        sky: (id % 3) as u8,
    }
}

#[derive(Clone, Copy)]
struct Camera {
    target: V,
    distance: f32,
    az: f32,
    el: f32,
}
impl Camera {
    fn pos(&self) -> V {
        V::new(
            self.target.x + self.distance * self.el.cos() * self.az.sin(),
            self.target.y + self.distance * self.el.sin(),
            self.target.z + self.distance * self.el.cos() * self.az.cos(),
        )
    }
    fn ray(&self, x: u32, y: u32, w: u32, h: u32) -> Ray {
        let p = self.pos();
        let f = (self.target - p).norm();
        let right = f.cross(V::new(0., 1., 0.)).norm();
        let up = right.cross(f);
        let aspect = w as f32 / h as f32;
        let sx = ((x as f32 + 0.5) / w as f32 * 2. - 1.) * aspect * 0.75;
        let sy = (1. - (y as f32 + 0.5) / h as f32 * 2.) * 0.75;
        Ray {
            o: p,
            d: (f + right * sx + up * sy).norm(),
        }
    }
}
fn sky(dir: V, id: u8) -> V {
    if id == 1 {
        let t = (dir.y + 1.) * 0.5;
        let base = V::new(0.015, 0.02, 0.08).lerp(V::new(0.03, 0.08, 0.28), t);
        let s = ((dir.x * 91. + dir.z * 47.).sin() * 43758.5).fract().abs();
        base + V::new(1., 0.8, 0.45) * (if s > 0.985 { 0.9 } else { 0. })
    } else if id == 2 {
        let t = (dir.y + 1.) * 0.5;
        V::new(0.25, 0.55, 0.95).lerp(V::new(0.75, 0.9, 1.), t)
    } else {
        let t = (dir.y + 1.) * 0.5;
        V::new(0.12, 0.08, 0.18).lerp(V::new(1., 0.32, 0.12), t)
    }
}
fn nearest(scene: &Scene, r: Ray) -> Option<Hit> {
    let local = Ray {
        o: ry(r.o, -scene.yaw),
        d: ry(r.d, -scene.yaw),
    };
    let mut best = None;
    for &c in &scene.cubes {
        if let Some(mut h) = hit_cube(c, local) {
            if best.as_ref().is_none_or(|b: &Hit| h.t < b.t) {
                h.point = ry(h.point, scene.yaw);
                h.normal = ry(h.normal, scene.yaw);
                best = Some(h);
            }
        }
    }
    best
}
fn visible(scene: &Scene, p: V, l: V) -> bool {
    let d = l - p;
    nearest(
        scene,
        Ray {
            o: p + d.norm() * EPS * 4.,
            d: d.norm(),
        },
    )
    .is_none_or(|h| h.t > d.len())
}
fn trace(scene: &Scene, r: Ray, depth: u32) -> V {
    let Some(h) = nearest(scene, r) else {
        return sky(r.d, scene.sky);
    };
    let mut out = h.material.emission;
    let base = texture(h.material, h.u, h.v);
    for &(lp, lc) in &scene.lights {
        let to = (lp - h.point).norm();
        if visible(scene, h.point, to) {
            let lam = h.normal.dot(to).max(0.);
            let view = (-r.d).norm();
            let half = (to + view).norm();
            out += base.mul(lc) * (lam * 0.85)
                + lc * (h.material.specular * h.normal.dot(half).max(0.).powf(32.));
        }
    }
    if depth < MAX_DEPTH {
        if h.material.reflect > 0. {
            let refl = r.d - h.normal * 2. * r.d.dot(h.normal);
            out += trace(
                scene,
                Ray {
                    o: h.point + h.normal * EPS * 3.,
                    d: refl.norm(),
                },
                depth + 1,
            ) * h.material.reflect;
        }
        if h.material.transparency > 0. {
            let mut n = h.normal;
            let mut cosi = (-r.d).dot(n).clamp(-1., 1.);
            let mut etai = 1.;
            let mut etat = h.material.ior;
            if cosi < 0. {
                cosi = -cosi;
                std::mem::swap(&mut etai, &mut etat);
                n = -n;
            }
            let eta = etai / etat;
            let k = 1. - eta * eta * (1. - cosi * cosi);
            if k >= 0. {
                let refr = r.d * eta + n * (eta * cosi - k.sqrt());
                out += trace(
                    scene,
                    Ray {
                        o: h.point - n * EPS * 3.,
                        d: refr.norm(),
                    },
                    depth + 1,
                ) * h.material.transparency;
            }
        }
    }
    out.clamp()
}
fn pixel(c: V) -> u32 {
    let q = |v: f32| -> u32 { (v.clamp(0., 1.).powf(1. / 2.2) * 255.) as u32 };
    (q(c.x) << 16) | (q(c.y) << 8) | q(c.z)
}

fn render_frame(
    scene: &Scene,
    cam: &Camera,
    w: u32,
    h: u32,
    fade: f32,
    buffer: &mut [u32],
    mut window: Option<&mut Window>,
) -> io::Result<(u128, V)> {
    let started = Instant::now();
    let mut avg = V::default();
    for y in 0..h {
        for x in 0..w {
            let c = trace(scene, cam.ray(x, y, w, h), 0) * fade;
            avg += c;
            buffer[(y * w + x) as usize] = pixel(c);
        }
        if let Some(window) = window.as_deref_mut() {
            if !window.is_open() {
                break;
            }
            // Present partial rows so the render is visibly progressive.
            if y % 4 == 3 || y + 1 == h {
                window
                    .update_with_buffer(buffer, w as usize, h as usize)
                    .map_err(|error| io::Error::other(error.to_string()))?;
            }
        }
    }
    Ok((started.elapsed().as_millis(), avg / (w * h) as f32))
}

fn write_ppm(path: &Path, buffer: &[u32], w: u32, h: u32) -> io::Result<()> {
    let mut f = BufWriter::new(File::create(path)?);
    writeln!(f, "P3\n{} {}\n255", w, h)?;
    for &rgb in buffer {
        writeln!(
            f,
            "{} {} {}",
            (rgb >> 16) & 255,
            (rgb >> 8) & 255,
            rgb & 255
        )?;
    }
    f.flush()
}

fn export_render(scene: &Scene, cam: &Camera, w: u32, h: u32, output: &Path) -> io::Result<()> {
    let mut buffer = vec![0; (w * h) as usize];
    let (ms, avg) = render_frame(scene, cam, w, h, 1., &mut buffer, None)?;
    write_ppm(output, &buffer, w, h)?;
    println!(
        "{} | {}x{} | {} ms | promedio {:0.2},{:0.2},{:0.2} | {}",
        scene.name,
        w,
        h,
        ms,
        avg.x,
        avg.y,
        avg.z,
        output.display()
    );
    Ok(())
}

fn transition_window(
    from: &Scene,
    to: &Scene,
    cam: &Camera,
    w: u32,
    h: u32,
    buffer: &mut [u32],
    window: &mut Window,
) -> io::Result<()> {
    for fade in [0.75, 0.25, 0., 0.35, 0.7, 1.] {
        let scene = if fade < 0.5 { from } else { to };
        render_frame(scene, cam, w, h, fade, buffer, Some(window))?;
        if !window.is_open() {
            break;
        }
    }
    let _ = (from, to);
    Ok(())
}

fn usage() {
    println!("Uso interactivo: cargo run -- [--scene 0|1|2] [--width N] [--height N]");
    println!("Exportación: cargo run -- --headless --scene 0 --output render.ppm [--width N] [--height N]");
    println!("Teclas: flechas/A-D orbitar, W/S elevar, +/- zoom, R girar, N cambiar escena, Escape salir.");
}
fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        usage();
        return Ok(());
    }
    let mut id = 0usize;
    let mut w = 320u32;
    let mut h = 240u32;
    let mut output = "render.ppm".to_string();
    let mut headless = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--scene" => {
                i += 1;
                id = args[i].parse().unwrap_or(0)
            }
            "--width" => {
                i += 1;
                w = args[i].parse().unwrap_or(320)
            }
            "--height" => {
                i += 1;
                h = args[i].parse().unwrap_or(240)
            }
            "--output" => {
                i += 1;
                output = args[i].clone()
            }
            "--render" | "--headless" => headless = true,
            "--interactive" => headless = false,
            _ => {}
        }
        i += 1;
    }
    let mut cam = Camera {
        target: V::new(0., 1., 0.),
        distance: 12.,
        az: 0.,
        el: 0.22,
    };
    let mut s = scene(id);
    if headless {
        export_render(&s, &cam, w, h, Path::new(&output))?;
        return Ok(());
    }
    let mut window = Window::new(
        "Proyecto 2 — Dioramas raytraced",
        w as usize,
        h as usize,
        WindowOptions::default(),
    )
    .map_err(|error| io::Error::other(error.to_string()))?;
    window.set_target_fps(60);
    let mut buffer = vec![0; (w * h) as usize];
    let mut dirty = true;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        if window.is_key_down(Key::Left) || window.is_key_down(Key::A) {
            cam.az -= 0.06;
            dirty = true;
        }
        if window.is_key_down(Key::Right) || window.is_key_down(Key::D) {
            cam.az += 0.06;
            dirty = true;
        }
        if window.is_key_down(Key::Up) || window.is_key_down(Key::W) {
            cam.el = (cam.el + 0.04).min(1.35);
            dirty = true;
        }
        if window.is_key_down(Key::Down) || window.is_key_down(Key::S) {
            cam.el = (cam.el - 0.04).max(-1.0);
            dirty = true;
        }
        if window.is_key_pressed(Key::Equal, KeyRepeat::No)
            || window.is_key_pressed(Key::NumPadPlus, KeyRepeat::No)
        {
            cam.distance = (cam.distance - 0.7).max(4.);
            dirty = true;
        }
        if window.is_key_pressed(Key::Minus, KeyRepeat::No)
            || window.is_key_pressed(Key::NumPadMinus, KeyRepeat::No)
        {
            cam.distance = (cam.distance + 0.7).min(30.);
            dirty = true;
        }
        if window.is_key_pressed(Key::R, KeyRepeat::No) {
            s.yaw += 0.15;
            dirty = true;
        }
        if window.is_key_pressed(Key::N, KeyRepeat::No) {
            let old = s.clone();
            id = (id + 1) % 3;
            let next = scene(id);
            transition_window(&old, &next, &cam, w, h, &mut buffer, &mut window)?;
            s = next;
            dirty = false;
        }
        if dirty {
            render_frame(&s, &cam, w, h, 1., &mut buffer, Some(&mut window))?;
            dirty = false;
        } else {
            window.update();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slab_hits_cube() {
        let c = Cube {
            min: V::new(-1., -1., -1.),
            max: V::new(1., 1., 1.),
            material: mat(Kind::Stone),
        };
        let h = hit_cube(
            c,
            Ray {
                o: V::new(0., 0., -3.),
                d: V::new(0., 0., 1.),
            },
        )
        .unwrap();
        assert!((h.t - 2.).abs() < 0.001);
        assert_eq!(h.normal, V::new(0., 0., -1.));
    }
    #[test]
    fn all_scenes_have_geometry() {
        for i in 0..3 {
            let s = scene(i);
            assert!(!s.cubes.is_empty());
            assert!(!s.lights.is_empty());
        }
    }
    #[test]
    fn refraction_material_is_present() {
        assert!(mat(Kind::Water).transparency > 0. && mat(Kind::Water).ior > 1.);
    }
    #[test]
    fn camera_orbit_preserves_distance_and_changes_ray() {
        let camera = Camera {
            target: V::new(0., 1., 0.),
            distance: 12.,
            az: 0.,
            el: 0.22,
        };
        let before = camera.pos();
        let mut rotated = camera;
        rotated.az += 0.5;
        let after = rotated.pos();
        assert!(((before - camera.target).len() - 12.).abs() < 0.001);
        assert!((after - before).len() > 0.1);
        assert_ne!(camera.ray(0, 0, 32, 24).d, rotated.ray(0, 0, 32, 24).d);
    }

    #[test]
    fn framebuffer_pixel_is_rgb888() {
        assert_eq!(pixel(V::new(1., 0.5, 0.)), 0xffba00);
    }
}
