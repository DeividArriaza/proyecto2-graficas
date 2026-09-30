use std::{
    env,
    fs::File,
    io::{self, BufWriter, Write},
    path::Path,
    time::Instant,
};

use minifb::{Key, KeyRepeat, Window, WindowOptions};
use rayon::prelude::*;

const EPS: f32 = 0.001;
const MAX_DEPTH: u32 = 3;
/// Fixed transition duration. Requests received while active are ignored.
const TRANSITION_DURATION: f32 = 0.8;

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
            V::new(0.12, 0.62, 0.10),
            0.12,
            0.02,
            0.,
            1.,
            V::default(),
        ),
        Kind::Brick => Material::new(
            k,
            V::new(0.82, 0.07, 0.03),
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
        Kind::Metal => Material::new(k, V::new(0.78, 0.82, 0.9), 0.7, 0.34, 0., 1., V::default()),
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
        Kind::Stone => Material::new(k, V::new(0.48, 0.52, 0.6), 0.18, 0.05, 0., 1., V::default()),
    }
}

#[derive(Clone, Copy)]
struct Cube {
    min: V,
    max: V,
    material: Material,
}
#[derive(Clone, Copy)]
struct Sphere {
    center: V,
    radius: f32,
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
fn hit_sphere(s: Sphere, r: Ray) -> Option<Hit> {
    let oc = r.o - s.center;
    let b = oc.dot(r.d);
    let c = oc.dot(oc) - s.radius * s.radius;
    let d = b * b - c;
    if d < 0. {
        return None;
    }
    let root = d.sqrt();
    let t = [-b - root, -b + root].into_iter().find(|t| *t > EPS)?;
    let point = r.o + r.d * t;
    let normal = (point - s.center) / s.radius;
    let u = 0.5 + normal.z.atan2(normal.x) / (2. * std::f32::consts::PI);
    let v = 0.5 - normal.y.asin() / std::f32::consts::PI;
    Some(Hit {
        t,
        point,
        normal,
        material: s.material,
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
    spheres: Vec<Sphere>,
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
fn sphere(out: &mut Vec<Sphere>, center: V, radius: f32, k: Kind) {
    out.push(Sphere {
        center,
        radius,
        material: mat(k),
    });
}
fn scene(id: usize) -> Scene {
    let mut c = Vec::new();
    let mut spheres = Vec::new();
    match id % 3 {
        0 => {
            for x in -6..=6 {
                for z in -5..=5 {
                    if x * x + z * z <= 38 {
                        cube(
                            &mut c,
                            V::new(x as f32, 0., z as f32),
                            V::new(1., 0.5, 1.),
                            Kind::Stone,
                        );
                    }
                }
            }
            cube(
                &mut c,
                V::new(0., 0.42, 0.),
                V::new(8.5, 0.22, 7.5),
                Kind::Water,
            );
            // Red hat-shaped Odyssey: broad hull, white deck, mast and balloon.
            cube(
                &mut c,
                V::new(0., 1.25, 0.3),
                V::new(6.2, 0.75, 2.6),
                Kind::Brick,
            );
            cube(
                &mut c,
                V::new(0., 1.7, 0.3),
                V::new(4.7, 0.35, 2.0),
                Kind::Metal,
            );
            cube(
                &mut c,
                V::new(0., 2.05, -0.05),
                V::new(2.8, 0.65, 1.45),
                Kind::Brick,
            );
            cube(
                &mut c,
                V::new(-2.5, 1.45, 0.3),
                V::new(1.0, 0.48, 1.7),
                Kind::Brick,
            );
            cube(
                &mut c,
                V::new(2.5, 1.45, 0.3),
                V::new(1.0, 0.48, 1.7),
                Kind::Brick,
            );
            cube(
                &mut c,
                V::new(0., 3.2, -0.1),
                V::new(0.22, 2.6, 0.22),
                Kind::Metal,
            );
            cube(
                &mut c,
                V::new(0., 3.45, -0.32),
                V::new(2.5, 1.5, 0.12),
                Kind::Brick,
            );
            sphere(&mut spheres, V::new(0., 5.1, -0.15), 1.45, Kind::Brick);
            sphere(&mut spheres, V::new(0., 5.1, 1.05), 0.42, Kind::Metal);
            for x in [-1.1, 1.1] {
                sphere(&mut spheres, V::new(x, 2.05, 1.05), 0.23, Kind::Star);
            }
        }
        1 => {
            // Floating planetoid with a bright, deliberately sparse orbital ring.
            sphere(&mut spheres, V::new(0., 1.9, 0.), 2.25, Kind::Grass);
            sphere(&mut spheres, V::new(-0.55, 3.35, 0.55), 0.62, Kind::Stone);
            sphere(&mut spheres, V::new(0.75, 3.45, -0.45), 0.36, Kind::Star);
            for i in 0..18 {
                let a = i as f32 * std::f32::consts::TAU / 18.;
                let p = V::new(a.cos() * 4.1, 2.0 + a.sin() * 0.75, a.sin() * 2.8);
                sphere(
                    &mut spheres,
                    p,
                    if i % 3 == 0 { 0.23 } else { 0.13 },
                    if i % 3 == 0 { Kind::Star } else { Kind::Metal },
                );
            }
            cube(
                &mut c,
                V::new(0., -0.5, 0.),
                V::new(3.0, 0.5, 3.0),
                Kind::Stone,
            );
        }
        _ => {
            for x in -6..=6 {
                for z in -5..=5 {
                    if x * x + z * z <= 42 {
                        cube(
                            &mut c,
                            V::new(x as f32, 0., z as f32),
                            V::new(1., 0.5, 1.),
                            Kind::Grass,
                        );
                    }
                }
            }
            // Bright stone castle with a readable arch, towers, pipes and coin trail.
            cube(
                &mut c,
                V::new(0., 2.0, -2.7),
                V::new(7.2, 4., 0.9),
                Kind::Stone,
            );
            cube(
                &mut c,
                V::new(0., 1.2, -3.2),
                V::new(1.45, 2.3, 0.2),
                Kind::Brick,
            );
            for x in [-3.1, 3.1] {
                cube(
                    &mut c,
                    V::new(x, 3.1, -2.7),
                    V::new(1.7, 6.2, 1.2),
                    Kind::Stone,
                );
                cube(
                    &mut c,
                    V::new(x, 6.3, -2.7),
                    V::new(2.25, 0.5, 1.35),
                    Kind::Brick,
                );
                for dx in [-0.55, 0.55] {
                    cube(
                        &mut c,
                        V::new(x + dx, 6.85, -2.7),
                        V::new(0.36, 0.55, 1.2),
                        Kind::Stone,
                    );
                }
            }
            for x in [-3.8, 3.8] {
                cube(
                    &mut c,
                    V::new(x, 1.25, 1.6),
                    V::new(1.0, 2.5, 1.0),
                    Kind::Pipe,
                );
                cube(
                    &mut c,
                    V::new(x, 2.65, 1.6),
                    V::new(1.65, 0.35, 1.65),
                    Kind::Pipe,
                );
            }
            for x in [-2.0, 0.0, 2.0] {
                cube(
                    &mut c,
                    V::new(x, 1.5, 0.25),
                    V::new(1., 1., 1.),
                    Kind::Brick,
                );
                sphere(&mut spheres, V::new(x, 3.0, 0.2), 0.3, Kind::Star);
            }
        }
    }
    let name = match id % 3 {
        0 => "Mario Odyssey",
        1 => "Mario Galaxy",
        _ => "NSMB Wii",
    };
    let lights = match id % 3 {
        0 => vec![
            (V::new(-5., 8., 4.), V::new(1.0, 0.78, 0.58)),
            (V::new(5., 4., 2.), V::new(0.42, 0.55, 1.0)),
            (V::new(0., 7., -4.), V::new(0.7, 0.2, 0.12)),
        ],
        1 => vec![
            (V::new(-4., 7., 5.), V::new(0.75, 0.85, 1.)),
            (V::new(4., 5., 1.), V::new(1., 0.45, 0.16)),
            (V::new(0., 8., -4.), V::new(0.35, 0.45, 1.)),
        ],
        _ => vec![
            (V::new(-5., 8., 4.), V::new(1., 0.92, 0.72)),
            (V::new(5., 5., 2.), V::new(0.42, 0.6, 1.)),
            (V::new(0., 7., -5.), V::new(0.65, 0.25, 0.16)),
        ],
    };
    Scene {
        name,
        cubes: c,
        spheres,
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
fn default_camera(id: usize) -> Camera {
    match id % 3 {
        0 => Camera {
            target: V::new(0., 2.35, 0.),
            distance: 14.5,
            az: 0.72,
            el: 0.38,
        },
        1 => Camera {
            target: V::new(0., 2.0, 0.),
            distance: 13.0,
            az: 0.72,
            el: 0.34,
        },
        _ => Camera {
            target: V::new(0., 2.6, -1.2),
            distance: 14.0,
            az: 0.68,
            el: 0.32,
        },
    }
}
fn sky(dir: V, id: u8) -> V {
    if id == 1 {
        let t = (dir.y + 1.) * 0.5;
        let base = V::new(0.015, 0.02, 0.08).lerp(V::new(0.03, 0.08, 0.28), t);
        let s = ((dir.x * 91. + dir.z * 47.).sin() * 43758.5).fract().abs();
        base + V::new(1., 0.85, 0.62) * (if s > 0.997 { 0.85 } else { 0. })
    } else if id == 2 {
        let t = (dir.y + 1.) * 0.5;
        V::new(0.25, 0.55, 0.95).lerp(V::new(0.75, 0.9, 1.), t)
    } else {
        let t = (dir.y + 1.) * 0.5;
        V::new(0.04, 0.1, 0.22).lerp(V::new(0.92, 0.38, 0.12), t)
    }
}
fn schlick(cosi: f32, etai: f32, etat: f32) -> f32 {
    let r0 = ((etat - etai) / (etat + etai)).powi(2);
    r0 + (1. - r0) * (1. - cosi).powi(5)
}
fn refract_direction(incident: V, normal: V, etai: f32, etat: f32) -> Option<V> {
    let cosi = (-incident).dot(normal).clamp(-1., 1.);
    let eta = etai / etat;
    let k = 1. - eta * eta * (1. - cosi * cosi);
    (k >= 0.).then(|| (incident * eta + normal * (eta * cosi - k.sqrt())).norm())
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
    for &s in &scene.spheres {
        if let Some(mut h) = hit_sphere(s, local) {
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
    let base = texture(h.material, h.u, h.v);
    // Low ambient prevents physically shadowed detail from collapsing to black.
    let mut out = h.material.emission + base * 0.16;
    for &(lp, lc) in &scene.lights {
        let to = (lp - h.point).norm();
        if visible(scene, h.point, lp) {
            let lam = h.normal.dot(to).max(0.);
            let view = (-r.d).norm();
            let half = (to + view).norm();
            out += base.mul(lc) * (lam * 0.85)
                + lc * (h.material.specular * h.normal.dot(half).max(0.).powf(32.));
        }
    }
    if depth < MAX_DEPTH {
        let mut reflect_weight = h.material.reflect.clamp(0., 1.);
        let mut refract_weight = 0.;
        let mut refracted = None;
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
            if let Some(refr) = refract_direction(r.d, n, etai, etat) {
                let fresnel = schlick(cosi, etai, etat);
                reflect_weight = reflect_weight.max(fresnel);
                refract_weight = (1. - fresnel) * h.material.transparency;
                refracted = Some((h.point - n * EPS * 3., refr));
            } else {
                reflect_weight = 1.;
            }
        }
        let local_weight = (1. - reflect_weight - refract_weight).max(0.);
        out = out * local_weight;
        if reflect_weight > 0. {
            let refl = r.d - h.normal * 2. * r.d.dot(h.normal);
            out += trace(
                scene,
                Ray {
                    o: h.point + h.normal * EPS * 3.,
                    d: refl.norm(),
                },
                depth + 1,
            ) * reflect_weight;
        }
        if let Some((o, d)) = refracted {
            out += trace(scene, Ray { o, d }, depth + 1) * refract_weight;
        }
    }
    out.clamp()
}
fn pixel(c: V) -> u32 {
    let q = |v: f32| -> u32 { (v.clamp(0., 1.).powf(1. / 2.2) * 255.) as u32 };
    (q(c.x) << 16) | (q(c.y) << 8) | q(c.z)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WorldTransition {
    from: usize,
    to: usize,
    elapsed_ms: u32,
    duration_ms: u32,
}

impl WorldTransition {
    fn new(from: usize, to: usize) -> Self {
        Self {
            from: from % 3,
            to: to % 3,
            elapsed_ms: 0,
            duration_ms: (TRANSITION_DURATION * 1000.0) as u32,
        }
    }

    fn progress(self) -> f32 {
        (self.elapsed_ms as f32 / self.duration_ms as f32).clamp(0.0, 1.0)
    }

    /// Cubic smoothstep keeps the start/end velocity at zero.
    fn eased_progress(self) -> f32 {
        let t = self.progress();
        t * t * (3.0 - 2.0 * t)
    }

    fn advance(&mut self, elapsed_ms: u32) {
        self.elapsed_ms = self
            .elapsed_ms
            .saturating_add(elapsed_ms)
            .min(self.duration_ms);
    }

    fn finished(self) -> bool {
        self.progress() >= 1.0
    }
}

fn requested_world(current: usize, key: Option<Key>) -> Option<usize> {
    match key {
        Some(Key::Key1) => Some(0),
        Some(Key::Key2) => Some(1),
        Some(Key::Key3) => Some(2),
        Some(Key::N) => Some((current + 1) % 3),
        _ => None,
    }
}

fn mix_pixel(a: u32, b: u32, t: f32, black: f32) -> u32 {
    let channel = |shift: u32| {
        let av = ((a >> shift) & 255) as f32;
        let bv = ((b >> shift) & 255) as f32;
        ((av * (1.0 - t) + bv * t) * (1.0 - black)).round() as u32
    };
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

fn composite_transition(from: &[u32], to: &[u32], progress: f32, output: &mut [u32]) {
    let t = progress.clamp(0.0, 1.0);
    // A restrained black dip makes the temporal boundary legible without hiding either world.
    let black = (std::f32::consts::PI * t).sin().max(0.0) * 0.18;
    for ((dst, &a), &b) in output.iter_mut().zip(from).zip(to) {
        *dst = mix_pixel(a, b, t, black);
    }
}

fn render_world(
    scene: &Scene,
    cam: &Camera,
    w: u32,
    h: u32,
    buffer: &mut [u32],
) -> io::Result<u128> {
    render_frame(scene, cam, w, h, 1.0, buffer, None).map(|(ms, _)| ms)
}

fn render_transition(
    transition: &WorldTransition,
    w: u32,
    h: u32,
    output: &mut [u32],
    from_buffer: &mut [u32],
    to_buffer: &mut [u32],
) -> io::Result<u128> {
    let from_scene = scene(transition.from);
    let to_scene = scene(transition.to);
    let from_camera = default_camera(transition.from);
    let to_camera = default_camera(transition.to);
    let t = transition.eased_progress();
    let camera = Camera {
        target: from_camera.target.lerp(to_camera.target, t),
        distance: from_camera.distance * (1.0 - t) + to_camera.distance * t,
        az: from_camera.az * (1.0 - t) + to_camera.az * t,
        el: from_camera.el * (1.0 - t) + to_camera.el * t,
    };
    let mut from_scene = from_scene;
    let mut to_scene = to_scene;
    from_scene.yaw *= 1.0 - t;
    to_scene.yaw *= t;
    let started = Instant::now();
    render_world(&from_scene, &camera, w, h, from_buffer)?;
    render_world(&to_scene, &camera, w, h, to_buffer)?;
    composite_transition(from_buffer, to_buffer, t, output);
    Ok(started.elapsed().as_millis())
}

fn render_frame(
    scene: &Scene,
    cam: &Camera,
    w: u32,
    h: u32,
    fade: f32,
    buffer: &mut [u32],
    window: Option<&mut Window>,
) -> io::Result<(u128, V)> {
    let started = Instant::now();
    let row_avgs: Vec<V> = buffer
        .par_chunks_mut(w as usize)
        .enumerate()
        .map(|(y, row)| {
            let mut avg = V::default();
            for (x, dst) in row.iter_mut().enumerate() {
                let c = trace(scene, cam.ray(x as u32, y as u32, w, h), 0) * fade;
                avg += c;
                *dst = pixel(c);
            }
            avg
        })
        .collect();
    if let Some(window) = window {
        if !window.is_open() {
            return Ok((started.elapsed().as_millis(), V::default()));
        }
        window
            .update_with_buffer(buffer, w as usize, h as usize)
            .map_err(|error| io::Error::other(error.to_string()))?;
    }
    let avg = row_avgs.into_iter().fold(V::default(), |a, b| a + b);
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

fn write_png(path: &Path, buffer: &[u32], w: u32, h: u32) -> io::Result<()> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, w, h);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut png_writer = encoder
        .write_header()
        .map_err(|error| io::Error::other(error.to_string()))?;
    let mut bytes = Vec::with_capacity(buffer.len() * 3);
    for &rgb in buffer {
        bytes.extend_from_slice(&[
            ((rgb >> 16) & 255) as u8,
            ((rgb >> 8) & 255) as u8,
            (rgb & 255) as u8,
        ]);
    }
    png_writer
        .write_image_data(&bytes)
        .map_err(|error| io::Error::other(error.to_string()))
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

fn framebuffer_hash(buffer: &[u32]) -> u64 {
    buffer.iter().fold(1469598103934665603, |hash, pixel| {
        (hash ^ u64::from(*pixel)).wrapping_mul(1099511628211)
    })
}

fn benchmark(benchmark_scene: &Scene, cam: &Camera, w: u32, h: u32) -> io::Result<()> {
    let mut camera = *cam;
    let mut buffer = vec![0; (w * h) as usize];
    let mut total_ms = 0u128;
    let mut min_ms = u128::MAX;
    let mut max_ms = 0;
    let mut changed = 0;
    let mut previous = 0;
    for frame in 0..12 {
        camera.az += 0.12;
        camera.el = (camera.el + 0.01).min(1.2);
        let (ms, _) = render_frame(benchmark_scene, &camera, w, h, 1., &mut buffer, None)?;
        let hash = framebuffer_hash(&buffer);
        if frame > 0 && hash != previous {
            changed += 1;
        }
        previous = hash;
        total_ms += ms;
        min_ms = min_ms.min(ms);
        max_ms = max_ms.max(ms);
    }
    let avg_ms = total_ms as f64 / 12.0;
    println!(
        "BENCHMARK {} | {}x{} | frames=12 | changed_frames={} | avg_ms={:.1} | min_ms={} | max_ms={} | fps={:.2} | hash={:016x}",
        benchmark_scene.name,
        w,
        h,
        changed,
        avg_ms,
        min_ms,
        max_ms,
        1000.0 / avg_ms,
        previous
    );
    let mut transition = WorldTransition::new(0, 1);
    let mut transition_output = vec![0; (w * h) as usize];
    let mut transition_from = vec![0; (w * h) as usize];
    let mut transition_to = vec![0; (w * h) as usize];
    let transition_started = Instant::now();
    let mut transition_frames = 0;
    while !transition.finished() {
        transition.advance(1000 / 12);
        render_transition(
            &transition,
            w,
            h,
            &mut transition_output,
            &mut transition_from,
            &mut transition_to,
        )?;
        transition_frames += 1;
    }
    let transition_fps = transition_frames as f64 / transition_started.elapsed().as_secs_f64();
    println!(
        "TRANSITION_BENCHMARK Mario Odyssey -> Mario Galaxy | duration={:.1}s | frames={} | fps={:.2} | final={:016x}",
        TRANSITION_DURATION,
        transition_frames,
        transition_fps,
        framebuffer_hash(&transition_output)
    );
    Ok(())
}

fn transition_demo(dir: &Path, w: u32, h: u32) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let mut manifest = String::from("{\n  \"duration_seconds\": 0.8,\n  \"input_during_transition\": \"ignored\",\n  \"frames\": [\n");
    let mut first = true;
    let mut frame_no = 0;
    let mut timings = Vec::new();
    for (from, to) in [(0, 1), (1, 2)] {
        let mut transition = WorldTransition::new(from, to);
        let mut output = vec![0; (w * h) as usize];
        let mut from_buffer = vec![0; (w * h) as usize];
        let mut to_buffer = vec![0; (w * h) as usize];
        let mut phase_hashes = Vec::new();
        for elapsed in [0, 200, 400, 600, 800] {
            transition.elapsed_ms = elapsed;
            let started = Instant::now();
            render_transition(
                &transition,
                w,
                h,
                &mut output,
                &mut from_buffer,
                &mut to_buffer,
            )?;
            let ms = started.elapsed().as_millis();
            let hash = framebuffer_hash(&output);
            phase_hashes.push(hash);
            let name = format!("frame-{frame_no:02}.png");
            let path = dir.join(&name);
            write_png(&path, &output, w, h)?;
            if !first {
                manifest.push_str(",\n");
            }
            first = false;
            manifest.push_str(&format!(
                "    {{\"file\":\"{name}\",\"from\":\"{}\",\"to\":\"{}\",\"elapsed_ms\":{elapsed},\"render_ms\":{ms},\"hash\":\"{hash:016x}\"}}",
                scene(from).name,
                scene(to).name
            ));
            timings.push(ms);
            frame_no += 1;
        }
        assert!(phase_hashes.windows(2).all(|pair| pair[0] != pair[1]));
    }
    let total_ms: u128 = timings.iter().sum();
    let fps = frame_no as f64 / (total_ms.max(1) as f64 / 1000.0);
    manifest.push_str(&format!(
        "\n  ],\n  \"hashes_distinct_within_each_transition\": true,\n  \"fps\": {:.2},\n  \"final_world\": \"NSMB Wii\"\n}}\n",
        fps
    ));
    std::fs::write(dir.join("manifest.json"), manifest)?;
    println!(
        "TRANSITION_DEMO {} frames={} fps={fps:.2} manifest={}",
        dir.display(),
        frame_no,
        dir.join("manifest.json").display()
    );
    Ok(())
}

fn usage() {
    println!("Uso interactivo: cargo run -- [--scene 0|1|2] [--width N] [--height N]");
    println!("Exportación: cargo run -- --headless --scene 0 --output render.ppm [--width N] [--height N]");
    println!("Benchmark: cargo run --release -- --benchmark --scene 0 [--width N] [--height N]");
    println!("Demo headless: cargo run -- --transition-demo DIR [--width N] [--height N]");
    println!("Teclas: flechas/A-D orbitar, W/S elevar, +/- zoom, R girar, N/1/2/3 cambiar escena, Escape salir.");
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
    let mut output = None;
    let mut headless = false;
    let mut do_benchmark = false;
    let mut demo_dir = None;
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
                output = Some(args[i].clone())
            }
            "--render" | "--headless" => headless = true,
            "--benchmark" => do_benchmark = true,
            "--transition-demo" => {
                i += 1;
                demo_dir = Some(args[i].clone())
            }
            "--interactive" => headless = false,
            _ => {}
        }
        i += 1;
    }
    let mut cam = default_camera(id);
    let mut s = scene(id);
    if do_benchmark {
        return benchmark(&s, &cam, w, h);
    }
    if let Some(dir) = demo_dir {
        return transition_demo(Path::new(&dir), w, h);
    }
    if headless {
        let Some(output) = output else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "--headless requiere --output; no se crea render.ppm por defecto",
            ));
        };
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
    let mut transition_buffers = (vec![0; (w * h) as usize], vec![0; (w * h) as usize]);
    let mut transition = None;
    let mut dirty = true;
    let mut last_tick = Instant::now();
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let now = Instant::now();
        let dt = now.duration_since(last_tick).as_secs_f32().min(0.1);
        last_tick = now;
        if window.is_key_down(Key::Left) || window.is_key_down(Key::A) {
            cam.az -= 1.8 * dt;
            dirty = true;
        }
        if window.is_key_down(Key::Right) || window.is_key_down(Key::D) {
            cam.az += 1.8 * dt;
            dirty = true;
        }
        if window.is_key_down(Key::Up) || window.is_key_down(Key::W) {
            cam.el = (cam.el + 1.2 * dt).min(1.35);
            dirty = true;
        }
        if window.is_key_down(Key::Down) || window.is_key_down(Key::S) {
            cam.el = (cam.el - 1.2 * dt).max(-1.0);
            dirty = true;
        }
        if window.is_key_down(Key::Equal) || window.is_key_down(Key::NumPadPlus) {
            cam.distance = (cam.distance - 8.0 * dt).max(4.);
            dirty = true;
        }
        if window.is_key_down(Key::Minus) || window.is_key_down(Key::NumPadMinus) {
            cam.distance = (cam.distance + 8.0 * dt).min(30.);
            dirty = true;
        }
        if window.is_key_down(Key::R) {
            s.yaw += 1.5 * dt;
            dirty = true;
        }
        let requested = [Key::N, Key::Key1, Key::Key2, Key::Key3]
            .into_iter()
            .find(|key| window.is_key_pressed(*key, KeyRepeat::No));
        if transition.is_none() {
            if let Some(next_id) = requested_world(id, requested) {
                if next_id != id {
                    transition = Some(WorldTransition::new(id, next_id));
                }
            }
        }
        if let Some(mut active) = transition {
            active.advance((dt * 1000.0) as u32);
            render_transition(
                &active,
                w,
                h,
                &mut buffer,
                &mut transition_buffers.0,
                &mut transition_buffers.1,
            )?;
            window
                .update_with_buffer(&buffer, w as usize, h as usize)
                .map_err(|error| io::Error::other(error.to_string()))?;
            if active.finished() {
                id = active.to;
                s = scene(id);
                cam = default_camera(id);
                dirty = false;
                transition = None;
            } else {
                transition = Some(active);
            }
        } else if dirty {
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
    fn sphere_has_correct_near_hit_and_normal() {
        let sphere = Sphere {
            center: V::default(),
            radius: 1.,
            material: mat(Kind::Stone),
        };
        let hit = hit_sphere(
            sphere,
            Ray {
                o: V::new(0., 0., -3.),
                d: V::new(0., 0., 1.),
            },
        )
        .unwrap();
        assert!((hit.t - 2.).abs() < EPS);
        assert_eq!(hit.normal, V::new(0., 0., -1.));
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

    #[test]
    fn framebuffer_changes_when_camera_moves() {
        let scene = scene(1);
        let mut a = Camera {
            target: V::new(0., 1., 0.),
            distance: 12.,
            az: 0.,
            el: 0.22,
        };
        let mut first = vec![0; 24 * 16];
        let mut second = vec![0; 24 * 16];
        render_frame(&scene, &a, 24, 16, 1., &mut first, None).unwrap();
        a.az += 0.35;
        render_frame(&scene, &a, 24, 16, 1., &mut second, None).unwrap();
        assert_ne!(framebuffer_hash(&first), framebuffer_hash(&second));
    }

    #[test]
    fn transition_state_has_bounded_smooth_progress_and_finishes() {
        let mut transition = WorldTransition::new(0, 1);
        assert_eq!(transition.progress(), 0.0);
        assert_eq!(transition.eased_progress(), 0.0);
        transition.advance(200);
        assert!((transition.progress() - 0.25).abs() < 0.001);
        assert!(transition.eased_progress() > 0.0 && transition.eased_progress() < 1.0);
        assert_ne!(transition.eased_progress(), transition.progress());
        transition.advance(10_000);
        assert_eq!(transition.progress(), 1.0);
        assert_eq!(transition.eased_progress(), 1.0);
        assert!(transition.finished());
    }

    #[test]
    fn direct_and_cyclic_world_selection_are_deterministic() {
        assert_eq!(requested_world(0, Some(Key::N)), Some(1));
        assert_eq!(requested_world(2, Some(Key::N)), Some(0));
        assert_eq!(requested_world(0, Some(Key::Key1)), Some(0));
        assert_eq!(requested_world(0, Some(Key::Key2)), Some(1));
        assert_eq!(requested_world(0, Some(Key::Key3)), Some(2));
        assert_eq!(requested_world(0, None), None);
    }

    #[test]
    fn transition_blends_both_worlds_without_an_instant_jump() {
        let from = vec![0x10_20_30; 4];
        let to = vec![0xe0_d0_c0; 4];
        let mut start = vec![0; 4];
        let mut middle = vec![0; 4];
        let mut end = vec![0; 4];
        composite_transition(&from, &to, 0.0, &mut start);
        composite_transition(&from, &to, 0.5, &mut middle);
        composite_transition(&from, &to, 1.0, &mut end);
        assert_eq!(start, from);
        assert_eq!(end, to);
        assert_ne!(framebuffer_hash(&start), framebuffer_hash(&middle));
        assert_ne!(framebuffer_hash(&middle), framebuffer_hash(&end));
        assert_ne!(start, end);
    }

    #[test]
    fn shadows_use_light_position() {
        let blocker = Cube {
            min: V::new(-0.5, -0.5, 2.),
            max: V::new(0.5, 0.5, 3.),
            material: mat(Kind::Stone),
        };
        let scene = Scene {
            name: "test",
            cubes: vec![blocker],
            spheres: vec![],
            lights: vec![(V::new(0., 0., 5.), V::new(1., 1., 1.))],
            yaw: 0.,
            sky: 0,
        };
        assert!(!visible(&scene, V::new(0., 0., 0.), V::new(0., 0., 5.)));
        assert!(visible(&scene, V::new(2., 0., 0.), V::new(0., 0., 5.)));
    }

    #[test]
    fn fresnel_and_total_internal_reflection_are_present() {
        assert!((schlick(1., 1., 1.5) - 0.04).abs() < 0.001);
        assert!(refract_direction(V::new(0., -1., 0.), V::new(0., 1., 0.), 1., 1.5).is_some());
        assert!(refract_direction(V::new(0.95, 0.312, 0.), V::new(0., 1., 0.), 1.5, 1.).is_none());
    }
}
