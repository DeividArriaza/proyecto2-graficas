use std::{
    env,
    fs::File,
    io::{self, BufWriter, Write},
    path::Path,
    time::Instant,
};

use minifb::{Key, KeyRepeat, Scale, ScaleMode, Window, WindowOptions};
use rayon::prelude::*;

mod audio;
mod transitions;
mod worlds;

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
    Planet,
    Brick,
    Pipe,
    Metal,
    Cloud,
    Dark,
    Water,
    Star,
    Stone,
    Sand,
    Sandstone,
    Cactus,
    StuccoTeal,
    StuccoYellow,
    StuccoMagenta,
    Skin,
    Castle,
    Roof,
    Path,
    Bark,
    LakeBed,
    LakeWater,
}
const MATERIALS: [Kind; 23] = [
    Kind::Grass,
    Kind::Planet,
    Kind::Brick,
    Kind::Pipe,
    Kind::Metal,
    Kind::Cloud,
    Kind::Dark,
    Kind::Water,
    Kind::Star,
    Kind::Stone,
    Kind::Sand,
    Kind::Sandstone,
    Kind::Cactus,
    Kind::StuccoTeal,
    Kind::StuccoYellow,
    Kind::StuccoMagenta,
    Kind::Skin,
    Kind::Castle,
    Kind::Roof,
    Kind::Path,
    Kind::Bark,
    Kind::LakeBed,
    Kind::LakeWater,
];
impl Kind {
    fn name(self) -> &'static str {
        match self {
            Self::Grass => "grass",
            Self::Planet => "planet",
            Self::Brick => "brick",
            Self::Pipe => "pipe",
            Self::Metal => "metal",
            Self::Cloud => "cloud",
            Self::Dark => "dark",
            Self::Water => "water",
            Self::Star => "star",
            Self::Stone => "stone",
            Self::Sand => "sand",
            Self::Sandstone => "sandstone",
            Self::Cactus => "cactus",
            Self::StuccoTeal => "stucco-teal",
            Self::StuccoYellow => "stucco-yellow",
            Self::StuccoMagenta => "stucco-magenta",
            Self::Skin => "skin",
            Self::Castle => "castle",
            Self::Roof => "roof",
            Self::Path => "path",
            Self::Bark => "bark",
            Self::LakeBed => "lake-bed",
            Self::LakeWater => "lake-water",
        }
    }
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
        Kind::LakeWater => Material::new(
            k,
            V::new(0.025, 0.25, 0.92),
            0.30,
            0.10,
            0.32,
            1.33,
            V::default(),
        ),
        Kind::LakeBed => Material::new(k, V::new(0.04, 0.22, 0.68), 0.02, 0., 0., 1., V::default()),
        Kind::Skin => Material::new(k, V::new(0.94, 0.70, 0.40), 0.08, 0., 0., 1., V::default()),
        Kind::Castle => Material::new(k, V::new(0.91, 0.88, 0.79), 0.10, 0., 0., 1., V::default()),
        Kind::Roof => Material::new(
            k,
            V::new(0.72, 0.025, 0.07),
            0.16,
            0.025,
            0.,
            1.,
            V::default(),
        ),
        Kind::Path => Material::new(k, V::new(0.83, 0.68, 0.42), 0.03, 0., 0., 1., V::default()),
        Kind::Bark => Material::new(k, V::new(0.30, 0.13, 0.045), 0.05, 0., 0., 1., V::default()),
        Kind::Grass => Material::new(
            k,
            V::new(0.12, 0.62, 0.10),
            0.12,
            0.02,
            0.,
            1.,
            V::default(),
        ),
        Kind::Planet => Material::new(
            k,
            V::new(0.10, 0.48, 0.18),
            0.22,
            0.08,
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
        Kind::Cloud => Material::new(k, V::new(0.94, 0.96, 1.0), 0.65, 0.2, 0., 1., V::default()),
        Kind::Dark => Material::new(
            k,
            V::new(0.025, 0.035, 0.075),
            0.5,
            0.22,
            0.,
            1.,
            V::default(),
        ),
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
        Kind::Sand => Material::new(k, V::new(0.89, 0.29, 0.12), 0.04, 0., 0., 1., V::default()),
        Kind::Sandstone => {
            Material::new(k, V::new(0.68, 0.24, 0.12), 0.08, 0., 0., 1., V::default())
        }
        Kind::Cactus => Material::new(
            k,
            V::new(0.12, 0.43, 0.19),
            0.15,
            0.01,
            0.,
            1.,
            V::default(),
        ),
        Kind::StuccoTeal => {
            Material::new(k, V::new(0.02, 0.58, 0.51), 0.10, 0., 0., 1., V::default())
        }
        Kind::StuccoYellow => {
            Material::new(k, V::new(0.98, 0.77, 0.12), 0.06, 0., 0., 1., V::default())
        }
        Kind::StuccoMagenta => Material::new(
            k,
            V::new(0.73, 0.06, 0.38),
            0.13,
            0.01,
            0.,
            1.,
            V::default(),
        ),
    }
}

#[derive(Clone, Copy)]
struct Cube {
    min: V,
    max: V,
    material: Material,
    ship: bool,
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
        Kind::LakeWater => m.albedo * (0.88 + 0.12 * (u * 15. + (v * 12.).sin()).sin().abs()),
        Kind::LakeBed => m.albedo * (0.82 + 0.18 * (u * 19. + v * 23.).sin().abs()),
        Kind::Skin => m.albedo * (0.96 + 0.04 * (u * 53. + v * 29.).sin().abs()),
        Kind::Castle => {
            let mortar =
                (v * 5.).fract() < 0.06 || (u * 4. + (v * 5.).floor() * 0.5).fract() < 0.04;
            m.albedo
                * if mortar {
                    0.74
                } else {
                    0.95 + 0.05 * (u * 71. + v * 83.).sin().abs()
                }
        }
        Kind::Roof => {
            m.albedo
                * if (v * 8.).fract() < 0.09 {
                    0.65
                } else {
                    0.90 + 0.1 * (u * 12.).sin().abs()
                }
        }
        Kind::Path => m.albedo * (0.86 + 0.14 * (u * 97. + v * 61.).sin().abs()),
        Kind::Bark => m.albedo * (0.65 + 0.35 * (u * 23. + (v * 9.).sin()).sin().abs()),
        Kind::Brick => {
            let mortar =
                (v * 6.).fract() < 0.07 || (u * 4. + (v * 6.).floor() % 2. * 0.5).fract() < 0.05;
            if mortar {
                m.albedo * 0.42
            } else if (x + y) % 2 == 0 {
                m.albedo * 1.15
            } else {
                m.albedo * 0.72
            }
        }
        Kind::Grass => {
            let n = ((u * 37.0).sin() * (v * 91.0).cos()).abs();
            m.albedo * (0.78 + n * 0.35)
        }
        Kind::Planet => {
            let continents = ((u * 11.0).sin() * (v * 7.0).cos()
                + (u * 23.0 + v * 5.0).sin() * 0.45
                + (v * 19.0).cos() * 0.25)
                .clamp(-1.0, 1.0);
            let ocean = V::new(0.05, 0.22, 0.38);
            let land = V::new(0.12, 0.62, 0.16);
            ocean.lerp(land, (continents + 1.0) * 0.5)
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
        Kind::Cloud => m.albedo * (0.92 + 0.08 * (u * 17. + v * 11.).sin().abs()),
        Kind::Dark => m.albedo * (0.75 + 0.25 * (u * 13. + v * 7.).sin().abs()),
        Kind::Water => m.albedo * (0.85 + 0.15 * (u * 20.).sin().abs()),
        Kind::Star => m.albedo * (0.9 + 0.1 * (u * 30.).sin().abs()),
        Kind::Stone => {
            if (x + y) % 3 == 0 {
                m.albedo * 1.3
            } else {
                m.albedo * 0.8
            }
        }
        Kind::Sand => {
            let grain = (u * 173. + (v * 97.).sin() * 11.).sin().abs();
            let ripple = (u * 24. + v * 5.).sin();
            m.albedo * (0.86 + grain * 0.1 + ripple * 0.04)
        }
        Kind::Sandstone => {
            let strata = (v * 36.).sin().abs();
            m.albedo * (0.75 + strata * 0.25)
        }
        Kind::Cactus => {
            let ribs = (u * 28.).sin().abs();
            m.albedo * (0.65 + ribs * 0.35)
        }
        Kind::StuccoTeal => m.albedo * (0.84 + 0.16 * (u * 83. + v * 61.).sin().abs()),
        Kind::StuccoYellow => {
            m.albedo * (0.88 + 0.12 * (u * 113.).sin().abs() * (v * 73.).cos().abs())
        }
        Kind::StuccoMagenta => m.albedo * (0.82 + 0.18 * (u * 47. - v * 97.).cos().abs()),
    }
    .clamp()
}

#[derive(Clone)]
struct Scene {
    name: &'static str,
    cubes: Vec<Cube>,
    bvh: Vec<BvhNode>,
    spheres: Vec<Sphere>,
    lights: Vec<(V, V)>,
    yaw: f32,
    sky: u8,
    moon_fill: f32,
}

#[derive(Clone)]
struct BvhNode {
    min: V,
    max: V,
    start: usize,
    count: usize,
    children: Option<(usize, usize)>,
}

fn build_bvh(cubes: &mut [Cube]) -> Vec<BvhNode> {
    fn build(cubes: &mut [Cube], start: usize, nodes: &mut Vec<BvhNode>) -> usize {
        let mut min = V::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut max = -min;
        for cube in cubes.iter() {
            min = V::new(
                min.x.min(cube.min.x),
                min.y.min(cube.min.y),
                min.z.min(cube.min.z),
            );
            max = V::new(
                max.x.max(cube.max.x),
                max.y.max(cube.max.y),
                max.z.max(cube.max.z),
            );
        }
        let index = nodes.len();
        nodes.push(BvhNode {
            min,
            max,
            start,
            count: cubes.len(),
            children: None,
        });
        if cubes.len() > 6 {
            let extent = max - min;
            let axis = if extent.x >= extent.y && extent.x >= extent.z {
                0
            } else if extent.y >= extent.z {
                1
            } else {
                2
            };
            let center = |cube: &Cube| match axis {
                0 => cube.min.x + cube.max.x,
                1 => cube.min.y + cube.max.y,
                _ => cube.min.z + cube.max.z,
            };
            cubes.sort_unstable_by(|a, b| center(a).total_cmp(&center(b)));
            let middle = cubes.len() / 2;
            let (left, right) = cubes.split_at_mut(middle);
            let a = build(left, start, nodes);
            let b = build(right, start + middle, nodes);
            nodes[index].children = Some((a, b));
        }
        index
    }
    let mut nodes = Vec::new();
    if !cubes.is_empty() {
        build(cubes, 0, &mut nodes);
    }
    nodes
}

fn bounds_hit(min: V, max: V, ray: Ray, limit: f32) -> bool {
    let (x0, x1) = axis_t(ray.o.x, ray.d.x, min.x, max.x);
    let (y0, y1) = axis_t(ray.o.y, ray.d.y, min.y, max.y);
    let (z0, z1) = axis_t(ray.o.z, ray.d.z, min.z, max.z);
    let enter = x0.max(y0).max(z0);
    let exit = x1.min(y1).min(z1);
    exit >= enter && exit >= EPS && enter <= limit
}

fn nearest_cube(scene: &Scene, ray: Ray, node: usize, best: &mut Option<Hit>) {
    let bound = &scene.bvh[node];
    if !bounds_hit(
        bound.min,
        bound.max,
        ray,
        best.as_ref().map_or(f32::INFINITY, |h| h.t),
    ) {
        return;
    }
    if let Some((a, b)) = bound.children {
        nearest_cube(scene, ray, a, best);
        nearest_cube(scene, ray, b, best);
    } else {
        for &cube in &scene.cubes[bound.start..bound.start + bound.count] {
            if let Some(hit) = hit_cube(cube, ray) {
                if best.as_ref().is_none_or(|previous| hit.t < previous.t) {
                    *best = Some(hit);
                }
            }
        }
    }
}

fn blocked_by_cube(scene: &Scene, ray: Ray, node: usize, distance: f32) -> bool {
    let bound = &scene.bvh[node];
    if !bounds_hit(bound.min, bound.max, ray, distance) {
        return false;
    }
    if let Some((a, b)) = bound.children {
        blocked_by_cube(scene, ray, a, distance) || blocked_by_cube(scene, ray, b, distance)
    } else {
        scene.cubes[bound.start..bound.start + bound.count]
            .iter()
            .any(|&cube| hit_cube(cube, ray).is_some_and(|h| h.t <= distance))
    }
}
fn cube(out: &mut Vec<Cube>, center: V, size: V, k: Kind) {
    let h = size * 0.5;
    out.push(Cube {
        min: center - h,
        max: center + h,
        material: mat(k),
        ship: false,
    });
}
fn desert_house(out: &mut Vec<Cube>, center: V, cell: f32, body: Kind) {
    // Painted masonry blocks, not a smooth imported mesh. AABB domes are
    // stepped roof tiers inspired by Desierto1.png and Desierto2.png.
    for x in 0..4 {
        for y in 0..4 {
            for z in 0..3 {
                cube(
                    out,
                    center
                        + V::new(
                            (x as f32 - 1.5) * cell,
                            (y as f32 + 0.5) * cell,
                            (z as f32 - 1.) * cell,
                        ),
                    V::new(cell - 0.018, cell - 0.018, cell - 0.018),
                    if y == 3 { Kind::StuccoYellow } else { body },
                );
            }
        }
    }
    let front = center.z + cell * 1.5;
    // Door, inset pane and contrasting white window frames.
    cube(
        out,
        V::new(center.x, center.y + cell, front),
        V::new(cell * 0.95, cell * 2., 0.08),
        Kind::Cloud,
    );
    cube(
        out,
        V::new(center.x, center.y + cell * 0.92, front + 0.05),
        V::new(cell * 0.70, cell * 1.8, 0.05),
        Kind::Dark,
    );
    for side in [-1., 1.] {
        let x = center.x + side * cell * 1.2;
        cube(
            out,
            V::new(x, center.y + cell * 2.3, front),
            V::new(cell * 0.85, cell * 0.9, 0.08),
            Kind::Cloud,
        );
        cube(
            out,
            V::new(x, center.y + cell * 2.3, front + 0.06),
            V::new(cell * 0.60, cell * 0.65, 0.05),
            Kind::Sandstone,
        );
        cube(
            out,
            V::new(x, center.y + cell * 2.3, front + 0.095),
            V::new(0.045, cell * 0.65, 0.02),
            Kind::Cloud,
        );
        cube(
            out,
            V::new(x, center.y + cell * 2.3, front + 0.095),
            V::new(cell * 0.60, 0.045, 0.02),
            Kind::Cloud,
        );
    }
    // Patterned cornice and the turquoise/yellow stepped dome.
    for x in 0..8 {
        cube(
            out,
            V::new(
                center.x + (x as f32 - 3.5) * cell * 0.5,
                center.y + cell * 3.05,
                front + 0.04,
            ),
            V::new(cell * 0.42, 0.12, 0.08),
            if x % 2 == 0 {
                Kind::Cloud
            } else {
                Kind::StuccoMagenta
            },
        );
    }
    for (tier, width) in [(0, 4.4), (1, 3.8), (2, 2.8), (3, 1.6)] {
        cube(
            out,
            center + V::new(0., cell * 4. + (tier as f32 + 0.5) * 0.25, 0.),
            V::new(cell * width, 0.25, cell * width * 0.8),
            if tier % 2 == 0 {
                Kind::StuccoTeal
            } else {
                Kind::StuccoYellow
            },
        );
    }
    // Low front step made from three separate blocks.
    for x in [-1., 0., 1.] {
        cube(
            out,
            V::new(center.x + x * cell, 0.58, front + cell * 0.55),
            V::new(cell - 0.02, 0.22, cell),
            Kind::Sandstone,
        );
    }
}

fn sand_kingdom(out: &mut Vec<Cube>) {
    // A finite square cutaway, with the ship resting on its sandy surface.
    // Horizontal strata make the sides read as a miniature terrain block.
    for (y, height, kind) in [
        (-1.25, 0.55, Kind::Sandstone),
        (-0.75, 0.45, Kind::Sand),
        (-0.30, 0.45, Kind::Sandstone),
        (0.12, 0.30, Kind::Sand),
    ] {
        cube(out, V::new(0., y, 0.), V::new(12., height, 12.), kind);
    }
    // Visible seams between a coherent grid of terrain blocks (2 world units).
    for x in 0..6 {
        for z in 0..6 {
            cube(
                out,
                V::new(-5. + x as f32 * 2., 0.37, -5. + z as f32 * 2.),
                V::new(1.97, 0.20, 1.97),
                Kind::Sand,
            );
        }
    }

    desert_house(out, V::new(-4.7, 0.47, 0.), 0.55, Kind::StuccoTeal);
    desert_house(out, V::new(3.65, 0.47, -4.1), 0.50, Kind::StuccoMagenta);

    // Stepped ruin and broken columns along the rear edge, behind the ship.
    for tier in 0..3 {
        let width = 1.8 - tier as f32 * 0.5;
        cube(
            out,
            V::new(-1.8, 0.77 + tier as f32 * 0.6, -4.7),
            V::new(width, 0.6, width),
            Kind::Sandstone,
        );
    }
    for (x, height) in [(-0.6, 1.3), (0.6, 1.7)] {
        cube(
            out,
            V::new(x, 0.60, -4.4),
            V::new(1., 0.26, 1.),
            Kind::Sandstone,
        );
        cube(
            out,
            V::new(x, 0.73 + height * 0.5, -4.4),
            V::new(0.56, height, 0.56),
            Kind::Sandstone,
        );
        cube(
            out,
            V::new(x, 0.85 + height, -4.4),
            V::new(0.85, 0.25, 0.85),
            Kind::Sand,
        );
    }

    // Two branched voxel cacti frame the deck without covering the headlight.
    for (x, z, height) in [(-5., -3.2, 1.9), (4.8, -1.4, 2.5)] {
        cube(
            out,
            V::new(x, 0.47 + height * 0.5, z),
            V::new(0.42, height, 0.42),
            Kind::Cactus,
        );
        for (side, branch_y) in [(-1., 1.15), (1., 1.55)] {
            cube(
                out,
                V::new(x + side * 0.38, branch_y, z),
                V::new(0.8, 0.28, 0.32),
                Kind::Cactus,
            );
            cube(
                out,
                V::new(x + side * 0.66, branch_y + 0.26, z),
                V::new(0.28, 0.65, 0.32),
                Kind::Cactus,
            );
        }
    }
    // Small dunes and scattered ruin fragments remain inside the square base.
    for (x, z) in [(-0.6, 4.5), (3.8, 4.3), (4.8, 0.2)] {
        cube(out, V::new(x, 0.58, z), V::new(1.4, 0.22, 1.1), Kind::Sand);
        cube(out, V::new(x, 0.75, z), V::new(0.8, 0.15, 0.6), Kind::Sand);
    }
    cube(
        out,
        V::new(-2.8, 0.65, -4.9),
        V::new(0.6, 0.36, 0.45),
        Kind::Sandstone,
    );
    // A small oasis: patterned pool floor behind a refractive water block.
    for x in 0..4 {
        for z in 0..3 {
            cube(
                out,
                V::new(-4.7 + x as f32 * 0.5, 0.50, 3.0 + z as f32 * 0.5),
                V::new(0.48, 0.06, 0.48),
                if (x + z) % 2 == 0 {
                    Kind::Cloud
                } else {
                    Kind::StuccoTeal
                },
            );
        }
    }
    cube(
        out,
        V::new(-3.95, 0.63, 3.5),
        V::new(2., 0.20, 1.5),
        Kind::Water,
    );
    for z in [2.6, 4.4] {
        cube(
            out,
            V::new(-3.95, 0.58, z),
            V::new(2.35, 0.22, 0.20),
            Kind::Sandstone,
        );
    }
    for x in [-5.12, -2.78] {
        cube(
            out,
            V::new(x, 0.58, 3.5),
            V::new(0.20, 0.22, 2.),
            Kind::Sandstone,
        );
    }
}
fn scene(id: usize) -> Scene {
    scene_with_charge(id, 0.5)
}

fn scene_with_charge(id: usize, fill: f32) -> Scene {
    let mut c = Vec::new();
    let spheres = Vec::new();
    match id % 3 {
        0 => {
            // Odyssey: a deliberately voxel-first reconstruction of the hat ship.
            // The hull, brim, tall red cup and headlight are stacked AABBs; rounded
            // primitives are intentionally not used in this scene.
            // Lower cream keel and red hull, stepped from stern to bow.
            cube(
                &mut c,
                V::new(0., 0.62, -0.05),
                V::new(5.4, 0.30, 2.35),
                Kind::Cloud,
            );
            cube(
                &mut c,
                V::new(0., 0.88, 0.02),
                V::new(6.15, 0.32, 2.72),
                Kind::Brick,
            );
            cube(
                &mut c,
                V::new(0., 1.12, 0.10),
                V::new(7.05, 0.22, 3.28),
                Kind::Cloud,
            );
            cube(
                &mut c,
                V::new(0., 1.30, 0.20),
                V::new(5.95, 0.22, 2.62),
                Kind::Brick,
            );
            // Bow platform projects forward in three blocky steps.
            cube(
                &mut c,
                V::new(0., 1.26, 1.78),
                V::new(4.65, 0.18, 1.02),
                Kind::Cloud,
            );
            cube(
                &mut c,
                V::new(0., 1.43, 2.28),
                V::new(3.35, 0.16, 0.55),
                Kind::Brick,
            );
            cube(
                &mut c,
                V::new(0., 1.56, 2.62),
                V::new(1.85, 0.14, 0.34),
                Kind::Cloud,
            );

            // Wide rear wing/deck and stepped side skirts.
            for x in [-2.95, 2.95] {
                cube(
                    &mut c,
                    V::new(x, 1.22, -0.10),
                    V::new(0.62, 0.26, 2.30),
                    Kind::Cloud,
                );
                cube(
                    &mut c,
                    V::new(x * 0.93, 1.48, -0.12),
                    V::new(0.54, 0.16, 1.86),
                    Kind::Brick,
                );
            }

            // Tall red cup/cabin: three progressively smaller horizontal bands.
            cube(
                &mut c,
                V::new(0., 1.82, -0.18),
                V::new(3.85, 0.62, 2.18),
                Kind::Brick,
            );
            cube(
                &mut c,
                V::new(0., 2.19, -0.18),
                V::new(4.05, 0.13, 2.31),
                Kind::Cloud,
            );
            cube(
                &mut c,
                V::new(0., 2.53, -0.18),
                V::new(3.62, 0.58, 2.04),
                Kind::Brick,
            );
            cube(
                &mut c,
                V::new(0., 2.87, -0.18),
                V::new(3.80, 0.12, 2.17),
                Kind::Cloud,
            );
            cube(
                &mut c,
                V::new(0., 3.20, -0.18),
                V::new(3.28, 0.55, 1.82),
                Kind::Brick,
            );
            cube(
                &mut c,
                V::new(0., 3.53, -0.18),
                V::new(3.50, 0.13, 1.97),
                Kind::Cloud,
            );

            // White-framed front windows, set into the tall cup.
            for x in [-0.92, 0.92] {
                cube(
                    &mut c,
                    V::new(x, 2.48, 0.89),
                    V::new(0.70, 1.08, 0.10),
                    Kind::Cloud,
                );
                cube(
                    &mut c,
                    V::new(x, 2.48, 0.955),
                    V::new(0.45, 0.75, 0.06),
                    Kind::Dark,
                );
            }
            cube(
                &mut c,
                V::new(0., 2.48, 0.96),
                V::new(0.18, 1.10, 0.08),
                Kind::Cloud,
            );

            // Faceted voxel headlight: cream casing, red rim and refractive lens.
            cube(
                &mut c,
                V::new(0., 1.62, 2.88),
                V::new(1.42, 1.10, 0.34),
                Kind::Brick,
            );
            cube(
                &mut c,
                V::new(0., 1.62, 3.08),
                V::new(1.12, 0.84, 0.12),
                Kind::Cloud,
            );
            cube(
                &mut c,
                V::new(0., 1.62, 3.16),
                V::new(0.72, 0.58, 0.07),
                Kind::Water,
            );
            for (x, y) in [(-0.62, 1.62), (0.62, 1.62), (0., 2.08), (0., 1.16)] {
                cube(
                    &mut c,
                    V::new(x, y, 3.17),
                    V::new(0.20, 0.20, 0.08),
                    Kind::Metal,
                );
            }

            // Block rails around the bow and cabin deck.
            for x in [-2.55, -1.65, 1.65, 2.55] {
                cube(
                    &mut c,
                    V::new(x, 1.88, 1.72),
                    V::new(0.09, 0.86, 0.09),
                    Kind::Metal,
                );
            }
            for z in [0.88, 1.72] {
                cube(
                    &mut c,
                    V::new(0., 2.23, z),
                    V::new(5.10, 0.08, 0.08),
                    Kind::Metal,
                );
            }
            for x in [-2.55, 2.55] {
                cube(
                    &mut c,
                    V::new(x, 2.23, 1.30),
                    V::new(0.08, 0.08, 0.92),
                    Kind::Metal,
                );
            }

            // Rear mast, flowing block flag, stepped tail and twin voxel thrusters.
            cube(
                &mut c,
                V::new(-2.45, 2.86, -1.10),
                V::new(0.11, 3.25, 0.11),
                Kind::Metal,
            );
            cube(
                &mut c,
                V::new(-1.66, 4.15, -1.10),
                V::new(1.52, 0.34, 0.10),
                Kind::Brick,
            );
            cube(
                &mut c,
                V::new(-0.98, 4.00, -1.10),
                V::new(0.36, 0.24, 0.10),
                Kind::Brick,
            );
            cube(
                &mut c,
                V::new(0., 1.18, -1.78),
                V::new(3.50, 0.34, 1.08),
                Kind::Brick,
            );
            for x in [-1.22, 1.22] {
                cube(
                    &mut c,
                    V::new(x, 1.18, -2.32),
                    V::new(0.78, 0.62, 0.48),
                    Kind::Metal,
                );
                cube(
                    &mut c,
                    V::new(x, 1.18, -2.62),
                    V::new(0.46, 0.34, 0.18),
                    Kind::Star,
                );
            }

            // Dark pedestal for the Power Moon globe.
            cube(
                &mut c,
                V::new(0., 3.83, -0.18),
                V::new(1.18, 0.28, 1.04),
                Kind::Dark,
            );
            cube(
                &mut c,
                V::new(0., 4.08, -0.18),
                V::new(0.42, 0.28, 0.42),
                Kind::Metal,
            );

            // Stepped voxel sphere where the Odyssey stores Power Moons. Small
            // gaps between the cubes keep its block construction visible.
            let scale = 0.55 + fill.clamp(0., 1.) * 0.90;
            let globe_cell = 0.40 * scale;
            let globe_step = globe_cell;
            let globe_center = V::new(0., 4.22 + 2.5 * globe_cell, -0.18);
            for (layer, cells) in [
                (0, &[(0, 0)][..]),
                (1, &[(-1, 0), (0, -1), (0, 0), (0, 1), (1, 0)][..]),
                (
                    2,
                    &[
                        (-1, -1),
                        (-1, 0),
                        (-1, 1),
                        (0, -1),
                        (0, 0),
                        (0, 1),
                        (1, -1),
                        (1, 0),
                        (1, 1),
                    ][..],
                ),
                (3, &[(-1, 0), (0, -1), (0, 0), (0, 1), (1, 0)][..]),
                (4, &[(0, 0)][..]),
            ] {
                for &(x, z) in cells {
                    cube(
                        &mut c,
                        V::new(
                            globe_center.x + x as f32 * globe_step,
                            globe_center.y + (layer as f32 - 2.) * globe_step,
                            globe_center.z + z as f32 * globe_step,
                        ),
                        V::new(globe_cell, globe_cell, globe_cell),
                        Kind::Star,
                    );
                }
            }

            // Black cap and golden finial from the reference ship.
            cube(
                &mut c,
                V::new(0., globe_center.y + 2. * globe_step + 0.26, -0.18),
                V::new(0.46, 0.16, 0.46),
                Kind::Dark,
            );
            cube(
                &mut c,
                V::new(
                    0.,
                    globe_center.y + 2. * globe_step + globe_cell * 0.5 + 0.25,
                    -0.18,
                ),
                V::new(0.22, 0.22, 0.22),
                Kind::Star,
            );
            for cube in &mut c {
                cube.ship = true;
            }
            sand_kingdom(&mut c);
        }
        1 => worlds::galaxy(&mut c),
        _ => worlds::mario64(&mut c),
    }
    let name = match id % 3 {
        0 => "Mario Odyssey",
        1 => "Mario Galaxy",
        _ => "Super Mario 64",
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
    let bvh = build_bvh(&mut c);
    Scene {
        name,
        cubes: c,
        bvh,
        spheres,
        lights,
        yaw: 0.,
        sky: (id % 3) as u8,
        moon_fill: fill.clamp(0., 1.),
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
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
            target: V::new(0., 1.8, 0.),
            distance: 14.8,
            az: 0.83,
            el: 0.52,
        },
        1 => Camera {
            target: V::new(0., 3.0, 0.),
            distance: 9.8,
            az: 0.38,
            el: 0.34,
        },
        _ => Camera {
            target: V::new(0., 2.7, -0.2),
            distance: 18.0,
            az: 0.62,
            el: 0.55,
        },
    }
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    t * t * (3. - 2. * t)
}

#[derive(Clone, Copy, Default)]
struct LightRig {
    azimuth: f32,
    elevation: f32,
}
impl LightRig {
    fn position(self, id: usize) -> V {
        let base = match id % 3 {
            0 | 2 => V::new(-5., 8., 4.),
            _ => V::new(-4., 7., 5.),
        };
        if self.azimuth == 0. && self.elevation == 0. {
            return base;
        }
        let target = V::new(0., 1.5, 0.);
        let offset = base - target;
        let radius = offset.len();
        let elevation = ((offset.y / radius).asin() + self.elevation).clamp(0.12, 1.4);
        let azimuth = offset.z.atan2(offset.x) + self.azimuth;
        target
            + V::new(
                radius * elevation.cos() * azimuth.cos(),
                radius * elevation.sin(),
                radius * elevation.cos() * azimuth.sin(),
            )
    }
    fn apply(self, scene: &mut Scene) {
        scene.lights[0].0 = self.position(scene.sky as usize);
    }
}

struct MoonCharge {
    count: u32,
    fill: f32,
    from: f32,
    elapsed: f32,
}
impl MoonCharge {
    fn new(count: u32) -> Self {
        let count = count.min(20);
        let fill = count as f32 / 20.;
        Self {
            count,
            fill,
            from: fill,
            elapsed: 0.5,
        }
    }
    fn change(&mut self, delta: i32) {
        self.count = (self.count as i32 + delta).clamp(0, 20) as u32;
        self.from = self.fill;
        self.elapsed = 0.;
    }
    fn update(&mut self, dt: f32) -> bool {
        if self.elapsed >= 0.5 {
            return false;
        }
        self.elapsed = (self.elapsed + dt).min(0.5);
        self.fill = self.from * (1. - smoothstep(self.elapsed / 0.5))
            + self.count as f32 / 20. * smoothstep(self.elapsed / 0.5);
        true
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
        V::new(0.66, 0.77, 0.84).lerp(V::new(0.12, 0.48, 0.76), t)
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
    if !scene.bvh.is_empty() {
        nearest_cube(scene, local, 0, &mut best);
    }
    for &s in &scene.spheres {
        if let Some(h) = hit_sphere(s, local) {
            if best.as_ref().is_none_or(|b: &Hit| h.t < b.t) {
                best = Some(h);
            }
        }
    }
    best.map(|mut h| {
        h.point = ry(h.point, scene.yaw);
        h.normal = ry(h.normal, scene.yaw);
        h
    })
}
fn visible(scene: &Scene, p: V, l: V) -> bool {
    let d = l - p;
    let direction = d.norm();
    let ray = Ray {
        o: ry(p + direction * EPS * 4., -scene.yaw),
        d: ry(direction, -scene.yaw),
    };
    let distance = d.len();
    // Shadow rays only need one blocker, not the nearest shaded intersection.
    !(!scene.bvh.is_empty() && blocked_by_cube(scene, ray, 0, distance))
        && !scene
            .spheres
            .iter()
            .any(|&sphere| hit_sphere(sphere, ray).is_some_and(|hit| hit.t <= distance))
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
    let map = |v: f32| v.max(0.) * 1.15 / (1. + 0.15 * v.max(0.));
    let q = |v: f32| -> u32 { (map(v).clamp(0., 1.).powf(1. / 2.2) * 255.) as u32 };
    (q(c.x) << 16) | (q(c.y) << 8) | q(c.z)
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WorldTransition {
    from: usize,
    to: usize,
    elapsed_ms: u32,
    duration_ms: u32,
    from_camera: Camera,
    from_yaw: f32,
    moon_fill: f32,
    from_light: V,
    to_light: V,
}

impl WorldTransition {
    fn new(from: usize, to: usize) -> Self {
        Self::from_view(from, to, default_camera(from), 0.)
    }

    fn from_view(from: usize, to: usize, camera: Camera, yaw: f32) -> Self {
        Self {
            from: from % 3,
            to: to % 3,
            elapsed_ms: 0,
            duration_ms: (TRANSITION_DURATION * 1000.0) as u32,
            from_camera: camera,
            from_yaw: yaw,
            moon_fill: 0.5,
            from_light: LightRig::default().position(from),
            to_light: LightRig::default().position(to),
        }
    }

    fn progress(self) -> f32 {
        (self.elapsed_ms as f32 / self.duration_ms as f32).clamp(0.0, 1.0)
    }

    /// Cubic smoothstep keeps the start/end velocity at zero.
    fn eased_progress(self) -> f32 {
        smoothstep(self.progress())
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
    let black = (std::f32::consts::PI * t).sin().max(0.0) * 0.07;
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
    render_frame(scene, cam, w, h, 1.0, buffer).map(|(ms, _)| ms)
}

#[cfg(test)]
fn render_transition(
    transition: &WorldTransition,
    w: u32,
    h: u32,
    output: &mut [u32],
    _from_buffer: &mut [u32],
    _to_buffer: &mut [u32],
) -> io::Result<u128> {
    transitions::Renderer::new(transition).render(transition, w, h, output)
}
fn render_frame(
    scene: &Scene,
    cam: &Camera,
    w: u32,
    h: u32,
    fade: f32,
    buffer: &mut [u32],
) -> io::Result<(u128, V)> {
    render_frame_mode(scene, cam, w, h, fade, buffer, None)
}

fn render_frame_mode(
    scene: &Scene,
    cam: &Camera,
    w: u32,
    h: u32,
    fade: f32,
    buffer: &mut [u32],
    inspection: Option<Kind>,
) -> io::Result<(u128, V)> {
    let started = Instant::now();
    let row_avgs: Vec<V> = buffer
        .par_chunks_mut(w as usize)
        .enumerate()
        .map(|(y, row)| {
            let mut avg = V::default();
            for (x, dst) in row.iter_mut().enumerate() {
                let ray = cam.ray(x as u32, y as u32, w, h);
                let c = if let Some(kind) = inspection {
                    inspect_ray(scene, ray, kind)
                } else {
                    trace(scene, ray, 0)
                } * fade;
                avg += c;
                *dst = pixel(c);
            }
            avg
        })
        .collect();
    let avg = row_avgs.into_iter().fold(V::default(), |a, b| a + b);
    Ok((started.elapsed().as_millis(), avg / (w * h) as f32))
}

fn inspect_ray(scene: &Scene, ray: Ray, selected: Kind) -> V {
    let Some(hit) = nearest(scene, ray) else {
        return V::new(0.08, 0.10, 0.13);
    };
    let shade = 0.35 + 0.65 * hit.normal.dot(V::new(-0.4, 0.8, 0.5).norm()).max(0.);
    if hit.material.kind != selected {
        return V::new(0.20, 0.22, 0.24) * shade;
    }
    if hit.u < 0.035 || hit.u > 0.965 || hit.v < 0.035 || hit.v > 0.965 {
        V::new(1., 0.70, 0.05)
    } else {
        texture(hit.material, hit.u, hit.v) * shade
    }
}

fn scene_materials(scene: &Scene) -> Vec<Kind> {
    MATERIALS
        .iter()
        .copied()
        .filter(|kind| {
            scene.cubes.iter().any(|cube| cube.material.kind == *kind)
                || scene
                    .spheres
                    .iter()
                    .any(|sphere| sphere.material.kind == *kind)
        })
        .collect()
}

fn inspection_title(scene: &Scene, selected: Option<Kind>) -> String {
    if let Some(kind) = selected {
        let m = mat(kind);
        format!("{} | {} | albedo={:.2},{:.2},{:.2} spec={:.2} transparencia={:.2} reflejo={:.2} | Tab: siguiente, M: salir",
            scene.name, kind.name(), m.albedo.x, m.albedo.y, m.albedo.z, m.specular, m.transparency, m.reflect)
    } else {
        format!(
            "{} | M: inspeccionar materiales | N: siguiente mundo",
            scene.name
        )
    }
}

fn next_material(scene: &Scene, current: Option<Kind>) -> Option<Kind> {
    let kinds = scene_materials(scene);
    if kinds.is_empty() {
        return None;
    }
    let index = current
        .and_then(|kind| kinds.iter().position(|k| *k == kind))
        .map_or(0, |index| (index + 1) % kinds.len());
    Some(kinds[index])
}

fn hud_lines(
    scene: &Scene,
    selected: Option<Kind>,
    moons: &MoonCharge,
    rig: LightRig,
    transition: Option<&WorldTransition>,
) -> Vec<String> {
    let mut lines = vec![format!("{} | 1/2/3 N:ESCENA M:MATERIALES", scene.name)];
    if let Some(transition) = transition {
        let motif = match transition.from {
            0 => "DESPEGUE ODYSSEY",
            1 => "LAUNCH STAR",
            _ => "TUBERIA",
        };
        lines.push(format!("TRANSICION: {motif}"));
        return lines;
    }
    if let Some(kind) = selected {
        let kinds = scene_materials(scene);
        let index = kinds.iter().position(|k| *k == kind).unwrap_or(0) + 1;
        let m = mat(kind);
        lines.push(format!(
            "MATERIAL {index}/{}: {} | TAB/T:SIGUIENTE M:SALIR",
            kinds.len(),
            kind.name()
        ));
        lines.push(format!(
            "ALBEDO {:.2},{:.2},{:.2} SPEC {:.2} TRANSP {:.2} REFLEJO {:.2}",
            m.albedo.x, m.albedo.y, m.albedo.z, m.specular, m.transparency, m.reflect
        ));
        lines.push("DIAGNOSTICO: M PARA VER LUZ Y REFLEXION".into());
    } else {
        lines.push("TAB/T:INSPECCION | +/-:ZOOM WASD:CAMARA R:ROTAR".into());
    }
    if scene.sky == 0 {
        lines.push(format!(
            "ENERGILUNAS {}/20 | Q:QUITAR E:ANADIR | GLOBO {:.0}%",
            moons.count,
            moons.fill * 100.
        ));
    }
    lines.push(format!(
        "LUZ J/L:AZ I/K:ALT H:RESET | GIRO {:.0} ALT {:.0}",
        rig.azimuth.to_degrees(),
        rig.elevation.to_degrees()
    ));
    lines
}

// Original compact bitmap alphabet for the on-screen controls and material label.
fn glyph(c: char) -> [u8; 7] {
    match c.to_ascii_uppercase() {
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 14],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'J' => [7, 2, 2, 2, 2, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 21, 19, 17, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        ':' => [0, 4, 4, 0, 4, 4, 0],
        '.' => [0, 0, 0, 0, 0, 4, 4],
        ',' => [0, 0, 0, 0, 4, 4, 8],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        '+' => [0, 4, 4, 31, 4, 4, 0],
        '/' => [1, 2, 2, 4, 8, 8, 16],
        '%' => [17, 2, 4, 8, 17, 0, 0],
        '|' => [4, 4, 4, 4, 4, 4, 4],
        _ => [0; 7],
    }
}

fn draw_hud(buffer: &mut [u32], width: usize, height: usize, lines: &[String]) {
    if width == 0 || height == 0 {
        return;
    }
    let longest = lines.iter().map(|line| line.len()).max().unwrap_or(0);
    let scale = if width >= longest * 12 + 16 { 2 } else { 1 };
    let line_height = 9 * scale;
    let top = height.saturating_sub(lines.len() * line_height + 8);
    for pixel in &mut buffer[top * width..] {
        *pixel = 0x12_18_20;
    }
    for (line, text) in lines.iter().enumerate() {
        for (col, character) in text.chars().enumerate() {
            for (row, bits) in glyph(character).iter().enumerate() {
                for bit in 0..5 {
                    if bits & (1 << (4 - bit)) == 0 {
                        continue;
                    }
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let x = 4 + col * 6 * scale + bit * scale + dx;
                            let y = top + 4 + line * line_height + row * scale + dy;
                            if x < width && y < height {
                                buffer[y * width + x] = 0xff_e1_98;
                            }
                        }
                    }
                }
            }
        }
    }
}

fn scale_letterboxed(
    source: &[u32],
    source_width: usize,
    source_height: usize,
    target: &mut Vec<u32>,
    target_width: usize,
    target_height: usize,
) {
    target.clear();
    target.resize(target_width.saturating_mul(target_height), 0);
    if source_width == 0 || source_height == 0 || target_width == 0 || target_height == 0 {
        return;
    }

    let (scaled_width, scaled_height) = if target_width.saturating_mul(source_height)
        <= target_height.saturating_mul(source_width)
    {
        (
            target_width,
            (target_width * source_height / source_width).max(1),
        )
    } else {
        (
            (target_height * source_width / source_height).max(1),
            target_height,
        )
    };
    let offset_x = (target_width - scaled_width) / 2;
    let offset_y = (target_height - scaled_height) / 2;

    for y in 0..scaled_height {
        let source_y = y * source_height / scaled_height;
        let target_row = (offset_y + y) * target_width + offset_x;
        let source_row = source_y * source_width;
        for x in 0..scaled_width {
            let source_x = x * source_width / scaled_width;
            target[target_row + x] = source[source_row + source_x];
        }
    }
}

fn present_frame(
    window: &mut Window,
    source: &[u32],
    source_width: usize,
    source_height: usize,
    presentation: &mut Vec<u32>,
    hud: &[String],
) -> io::Result<(usize, usize)> {
    let (window_width, window_height) = window.get_size();
    if window_width == 0 || window_height == 0 {
        window.update();
        return Ok((window_width, window_height));
    }
    scale_letterboxed(
        source,
        source_width,
        source_height,
        presentation,
        window_width,
        window_height,
    );
    draw_hud(presentation, window_width, window_height, hud);
    window
        .update_with_buffer(presentation, window_width, window_height)
        .map_err(|error| io::Error::other(error.to_string()))?;
    Ok((window_width, window_height))
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

#[derive(Clone, Copy)]
struct FrameMetrics {
    mean: f64,
    stddev: f64,
    p05: f64,
    p95: f64,
}

fn frame_metrics(buffer: &[u32]) -> FrameMetrics {
    let mut values: Vec<f64> = buffer
        .iter()
        .map(|rgb| {
            0.2126 * f64::from((rgb >> 16) & 255)
                + 0.7152 * f64::from((rgb >> 8) & 255)
                + 0.0722 * f64::from(rgb & 255)
        })
        .collect();
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let stddev = (values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / values.len() as f64)
        .sqrt();
    values.sort_by(f64::total_cmp);
    FrameMetrics {
        mean,
        stddev,
        p05: values[values.len() * 5 / 100],
        p95: values[values.len() * 95 / 100],
    }
}

fn export_render(
    scene: &Scene,
    cam: &Camera,
    w: u32,
    h: u32,
    output: &Path,
    inspection: Option<Kind>,
    hud: Option<&[String]>,
) -> io::Result<()> {
    let mut buffer = vec![0; (w * h) as usize];
    let (ms, avg) = render_frame_mode(scene, cam, w, h, 1., &mut buffer, inspection)?;
    if let Some(hud) = hud {
        draw_hud(&mut buffer, w as usize, h as usize, hud);
    }
    if inspection.is_some() {
        println!("INSPECTION {}", inspection_title(scene, inspection));
    }
    if output
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
    {
        write_png(output, &buffer, w, h)?;
    } else {
        write_ppm(output, &buffer, w, h)?;
    }
    let metrics = frame_metrics(&buffer);
    println!(
        "{} | {}x{} | {} ms | promedio {:0.2},{:0.2},{:0.2} | hash={:016x} | luma mean={:.3} stddev={:.3} p05={:.3} p95={:.3} | {}",
        scene.name,
        w,
        h,
        ms,
        avg.x,
        avg.y,
        avg.z,
        framebuffer_hash(&buffer),
        metrics.mean,
        metrics.stddev,
        metrics.p05,
        metrics.p95,
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
        let (ms, _) = render_frame(benchmark_scene, &camera, w, h, 1., &mut buffer)?;
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
    let from = benchmark_scene.sky as usize;
    let to = (from + 1) % 3;
    let mut transition = WorldTransition::from_view(from, to, camera, benchmark_scene.yaw);
    transition.moon_fill = benchmark_scene.moon_fill;
    transition.from_light = benchmark_scene.lights[0].0;
    let mut transition_output = vec![0; (w * h) as usize];
    let transition_started = Instant::now();
    let mut prepared = transitions::Renderer::new(&transition);
    let mut transition_frames = 0;
    while !transition.finished() {
        transition.advance(1000 / 12);
        prepared.render(&transition, w, h, &mut transition_output)?;
        transition_frames += 1;
    }
    let transition_fps = transition_frames as f64 / transition_started.elapsed().as_secs_f64();
    println!(
        "TRANSITION_BENCHMARK {} -> {} | duration={:.1}s | frames={} | fps={:.2} | final={:016x}",
        benchmark_scene.name,
        scene(to).name,
        TRANSITION_DURATION,
        transition_frames,
        transition_fps,
        framebuffer_hash(&transition_output)
    );
    Ok(())
}

fn transition_demo(dir: &Path, w: u32, h: u32) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let mut manifest = String::from("{\n  \"duration_seconds\": 0.8,\n  \"input_during_transition\": \"ignored\",\n  \"prepared_scenes\": true,\n  \"minimum_resolution_scale\": 0.5,\n  \"frames\": [\n");
    let mut first = true;
    let mut frame_no = 0;
    let mut timings = Vec::new();
    let mut preparation_ms = 0;
    for (from, to) in [(0, 1), (1, 2), (2, 0)] {
        let mut transition = WorldTransition::new(from, to);
        let mut output = vec![0; (w * h) as usize];
        let preparation_started = Instant::now();
        let mut prepared = transitions::Renderer::new(&transition);
        preparation_ms += preparation_started.elapsed().as_millis();
        let mut phase_hashes = Vec::new();
        for elapsed in [0, 200, 400, 600, 800] {
            transition.elapsed_ms = elapsed;
            let started = Instant::now();
            prepared.render(&transition, w, h, &mut output)?;
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
            let (rw, rh) = transitions::resolution(w, h, transition.progress());
            manifest.push_str(&format!(
                "    {{\"file\":\"{name}\",\"from\":\"{}\",\"to\":\"{}\",\"elapsed_ms\":{elapsed},\"internal_resolution\":[{rw},{rh}],\"render_ms\":{ms},\"hash\":\"{hash:016x}\"}}",
                scene(from).name,
                scene(to).name
            ));
            timings.push(ms);
            frame_no += 1;
        }
        assert!(phase_hashes.windows(2).all(|pair| pair[0] != pair[1]));
    }
    let total_ms: u128 = timings.iter().sum::<u128>() + preparation_ms;
    let fps = frame_no as f64 / (total_ms.max(1) as f64 / 1000.0);
    manifest.push_str(&format!(
        "\n  ],\n  \"hashes_distinct_within_each_transition\": true,\n  \"fps\": {:.2},\n  \"final_world\": \"Mario Odyssey\"\n}}\n",
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
    println!("Inspección: --inspect-material sand|metal|water|stucco-teal (con --headless y --output, o interactivo)");
    println!("Estado: --moons 0..20 --light-azimuth GRADOS --light-elevation GRADOS; --hud incluye el panel en exportación headless.");
    println!("Audio: --mute, --audio-dir DIR (6 WAV propios), --export-audio DIR (síntesis original sin ventana). B silencia/activa.");
    println!("Pruebas opcionales: --audio-demo (ciclo de sonido sin ventana); --smoke-frames N (cerrar ventana normalmente tras N ticks).");
    println!("Teclas: flechas/A-D orbitar, W/S elevar, +/- zoom, R girar, M inspeccionar, Tab/T iniciar o recorrer materiales, J/L luz horizontal, I/K luz vertical, H restablecer luz, Q/E quitar/añadir energilunas, N/1/2/3 cambiar escena, Escape salir.");
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
    let mut inspection = None;
    let mut moon_count = 10;
    let mut rig = LightRig::default();
    let mut export_hud = false;
    let mut muted = false;
    let mut audio_dir = None;
    let mut audio_export = None;
    let mut smoke_frames = None;
    let mut audio_demo = false;
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
            "--moons" => {
                i += 1;
                moon_count = args
                    .get(i)
                    .and_then(|s| s.parse::<u32>().ok())
                    .filter(|n| *n <= 20)
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "--moons requiere un entero de 0 a 20",
                        )
                    })?;
            }
            "--light-azimuth" | "--light-elevation" => {
                let azimuth = args[i] == "--light-azimuth";
                i += 1;
                let degrees = args
                    .get(i)
                    .and_then(|s| s.parse::<f32>().ok())
                    .filter(|n| n.is_finite())
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "el angulo de luz debe ser finito, en grados",
                        )
                    })?;
                if azimuth {
                    rig.azimuth = degrees.to_radians();
                } else {
                    rig.elevation = degrees.to_radians().clamp(-0.7, 0.6);
                }
            }
            "--hud" => export_hud = true,
            "--mute" => muted = true,
            "--audio-demo" => audio_demo = true,
            "--smoke-frames" => {
                i += 1;
                smoke_frames = Some(
                    args.get(i)
                        .and_then(|v| v.parse::<u32>().ok())
                        .filter(|n| *n > 0)
                        .ok_or_else(|| {
                            io::Error::new(
                                io::ErrorKind::InvalidInput,
                                "--smoke-frames requiere un entero positivo",
                            )
                        })?,
                );
            }
            "--audio-dir" | "--export-audio" => {
                let exporting = args[i] == "--export-audio";
                i += 1;
                let dir = args
                    .get(i)
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidInput, "audio requiere un directorio")
                    })?
                    .clone();
                if exporting {
                    audio_export = Some(dir);
                } else {
                    audio_dir = Some(dir);
                }
            }
            "--inspect-material" => {
                i += 1;
                let name = args.get(i).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--inspect-material requiere un nombre",
                    )
                })?;
                inspection = Some(
                    *MATERIALS
                        .iter()
                        .find(|kind| kind.name() == name)
                        .ok_or_else(|| {
                            io::Error::new(io::ErrorKind::InvalidInput, "material desconocido")
                        })?,
                );
            }
            _ => {}
        }
        i += 1;
    }
    if let Some(dir) = audio_export {
        audio::export(Path::new(&dir))?;
        println!(
            "AUDIO_EXPORT {dir} | 3 ambientes originales de 12s + 3 efectos de 0.8s | PCM 22050 Hz"
        );
        return Ok(());
    }
    let default_audio = Path::new("assets/audio");
    if audio_dir.is_none()
        && audio::FILES
            .iter()
            .all(|name| default_audio.join(name).is_file())
    {
        audio_dir = Some(default_audio.to_string_lossy().into_owned());
    } else if audio_dir.is_none()
        && audio::FILES
            .iter()
            .any(|name| default_audio.join(name).is_file())
    {
        eprintln!("Audio: assets/audio está incompleto; coloca los seis WAV indicados en su README. Se usarán ambientes originales.");
    }
    if audio_demo {
        return audio::demo(audio_dir.as_deref().map(Path::new));
    }
    let mut cam = default_camera(id);
    let mut moons = MoonCharge::new(moon_count);
    let mut s = scene_with_charge(id, moons.fill);
    rig.apply(&mut s);
    if inspection.is_some_and(|kind| !scene_materials(&s).contains(&kind)) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "el material no existe en esta escena",
        ));
    }
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
        let hud = hud_lines(&s, inspection, &moons, rig, None);
        export_render(
            &s,
            &cam,
            w,
            h,
            Path::new(&output),
            inspection,
            export_hud.then_some(hud.as_slice()),
        )?;
        return Ok(());
    }
    let mut window = Window::new(
        "Proyecto 2 — Dioramas raytraced (320x240 interno, F11 no disponible en minifb)",
        w as usize,
        h as usize,
        WindowOptions {
            // Scale::X4 gives a 1280x960 default for the 320x240 renderer.
            // Proportional resizing is performed in Rust because minifb 0.26's
            // Wayland AspectRatioStretch path can access memory out of bounds.
            resize: true,
            scale: Scale::X4,
            scale_mode: ScaleMode::Stretch,
            ..WindowOptions::default()
        },
    )
    .map_err(|error| io::Error::other(error.to_string()))?;
    window.set_target_fps(60);
    let mut sound = audio::Audio::new(audio_dir.as_deref().map(Path::new), muted, id)?;
    window.set_title(&inspection_title(&s, inspection));
    let mut buffer = vec![0; (w * h) as usize];
    let mut presentation = Vec::new();
    let mut presented_size = (0, 0);
    let mut prepared_transition = None;
    let mut transition = None;
    let mut transition_started = Instant::now();
    let mut dirty = true;
    let mut last_tick = Instant::now();
    let mut ticks = 0u32;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let now = Instant::now();
        let elapsed = now.duration_since(last_tick);
        let dt = elapsed.as_secs_f32().min(0.1);
        last_tick = now;
        let previous_audio_label = sound.label();
        sound.tick();
        if window.is_key_pressed(Key::B, KeyRepeat::No) {
            sound.toggle();
        }
        if sound.label() != previous_audio_label {
            dirty = true;
        }
        if transition.is_none() {
            if window.is_key_pressed(Key::M, KeyRepeat::No) {
                inspection = if inspection.is_some() {
                    None
                } else {
                    scene_materials(&s).first().copied()
                };
                window.set_title(&inspection_title(&s, inspection));
                println!("{}", inspection_title(&s, inspection));
                dirty = true;
            }
            if window.is_key_pressed(Key::Tab, KeyRepeat::No)
                || window.is_key_pressed(Key::T, KeyRepeat::No)
            {
                inspection = next_material(&s, inspection);
                window.set_title(&inspection_title(&s, inspection));
                println!("{}", inspection_title(&s, inspection));
                dirty = true;
            }
            if s.sky == 0 {
                if window.is_key_pressed(Key::E, KeyRepeat::No) {
                    moons.change(1);
                }
                if window.is_key_pressed(Key::Q, KeyRepeat::No) {
                    moons.change(-1);
                }
            }
            if moons.update(elapsed.as_secs_f32()) && s.sky == 0 {
                let yaw = s.yaw;
                s = scene_with_charge(id, moons.fill);
                s.yaw = yaw;
                rig.apply(&mut s);
                dirty = true;
            }
            let azimuth_delta =
                i32::from(window.is_key_down(Key::L)) - i32::from(window.is_key_down(Key::J));
            let elevation_delta =
                i32::from(window.is_key_down(Key::I)) - i32::from(window.is_key_down(Key::K));
            if azimuth_delta != 0 || elevation_delta != 0 {
                rig.azimuth += azimuth_delta as f32 * dt;
                rig.elevation =
                    (rig.elevation + elevation_delta as f32 * dt * 0.7).clamp(-0.7, 0.6);
                rig.apply(&mut s);
                dirty = true;
            }
            if window.is_key_pressed(Key::H, KeyRepeat::No) {
                rig = LightRig::default();
                rig.apply(&mut s);
                dirty = true;
            }
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
        }
        let requested = [Key::N, Key::Key1, Key::Key2, Key::Key3]
            .into_iter()
            .find(|key| window.is_key_pressed(*key, KeyRepeat::No));
        if transition.is_none() {
            if let Some(next_id) = requested_world(id, requested) {
                if next_id != id {
                    inspection = None;
                    let mut active = WorldTransition::from_view(id, next_id, cam, s.yaw);
                    active.moon_fill = moons.fill;
                    active.from_light = s.lights[0].0;
                    active.to_light = rig.position(next_id);
                    prepared_transition = Some(transitions::Renderer::new(&active));
                    sound.transition(id);
                    transition = Some(active);
                    transition_started = Instant::now();
                }
            }
        }
        let mut hud = hud_lines(&s, inspection, &moons, rig, transition.as_ref());
        hud.push(sound.label().into());
        if let Some(mut active) = transition {
            active.elapsed_ms = transition_started
                .elapsed()
                .as_millis()
                .min(u128::from(active.duration_ms)) as u32;
            prepared_transition
                .as_mut()
                .expect("active transition must be prepared")
                .render(&active, w, h, &mut buffer)?;
            presented_size = present_frame(
                &mut window,
                &buffer,
                w as usize,
                h as usize,
                &mut presentation,
                &hud,
            )?;
            if active.finished() {
                id = active.to;
                sound.arrive(id);
                s = prepared_transition
                    .take()
                    .expect("completed transition must be prepared")
                    .into_destination();
                cam = default_camera(id);
                window.set_title(&inspection_title(&s, None));
                dirty = true;
                transition = None;
                prepared_transition = None;
            } else {
                transition = Some(active);
            }
        } else if dirty {
            render_frame_mode(&s, &cam, w, h, 1., &mut buffer, inspection)?;
            presented_size = present_frame(
                &mut window,
                &buffer,
                w as usize,
                h as usize,
                &mut presentation,
                &hud,
            )?;
            dirty = false;
        } else if window.get_size() != presented_size {
            presented_size = present_frame(
                &mut window,
                &buffer,
                w as usize,
                h as usize,
                &mut presentation,
                &hud,
            )?;
        } else {
            window.update();
        }
        ticks = ticks.saturating_add(1);
        if smoke_frames.is_some_and(|limit| ticks >= limit) {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reference_worlds_are_voxel_and_keep_their_travel_landmarks() {
        let galaxy = scene(1);
        let castle = scene(2);
        for world in [&galaxy, &castle] {
            assert!(world.spheres.is_empty());
            assert!(world.cubes.len() > 300);
            assert!(world
                .cubes
                .iter()
                .all(|c| c.max.x > c.min.x && c.max.y > c.min.y && c.max.z > c.min.z));
        }
        assert_eq!(castle.name, "Super Mario 64");
        let near = |cube: &Cube, point: V| ((cube.min + cube.max) * 0.5 - point).len() < 0.15;
        assert!(galaxy
            .cubes
            .iter()
            .any(|c| c.material.kind == Kind::Star && near(c, worlds::GALAXY_EXIT)));
        assert!(galaxy.cubes.iter().any(|c| c.material.kind == Kind::Skin));
        assert!(castle
            .cubes
            .iter()
            .any(|c| c.material.kind == Kind::Roof && c.max.y > 9.));
        assert!(castle
            .cubes
            .iter()
            .any(|c| c.material.kind == Kind::LakeWater));
        assert!(castle
            .cubes
            .iter()
            .any(|c| c.material.kind == Kind::LakeBed));
        assert!(castle.cubes.iter().any(|c| c.material.kind == Kind::Dark
            && near(c, worlds::CASTLE_EXIT - V::new(0., 0.12, 0.))));
    }
    #[test]
    fn slab_hits_cube() {
        let c = Cube {
            min: V::new(-1., -1., -1.),
            max: V::new(1., 1., 1.),
            material: mat(Kind::Stone),
            ship: false,
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
    fn odyssey_is_a_cube_dominant_voxel_ship() {
        let odyssey = scene(0);
        assert!(odyssey.cubes.len() >= 65);
        assert!(odyssey.spheres.is_empty());
        // The tall cabin and the forward headlight both extend above/forward of the hull.
        assert!(odyssey.cubes.iter().any(|cube| cube.max.y > 5.7));
        assert!(odyssey.cubes.iter().any(|cube| cube.max.z > 3.1));
        assert!(
            odyssey
                .cubes
                .iter()
                .filter(|cube| cube.min.y > 4.15 && cube.material.kind == Kind::Star)
                .count()
                >= 21
        );
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
        assert_eq!(pixel(V::new(1., 0.5, 0.)), 0xffbf00);
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
        render_frame(&scene, &a, 24, 16, 1., &mut first).unwrap();
        a.az += 0.35;
        render_frame(&scene, &a, 24, 16, 1., &mut second).unwrap();
        assert_ne!(framebuffer_hash(&first), framebuffer_hash(&second));
    }

    #[test]
    fn letterbox_scaling_preserves_aspect_ratio() {
        let source = vec![0x11_22_33; 4 * 3];
        let mut target = Vec::new();
        scale_letterboxed(&source, 4, 3, &mut target, 16, 9);
        assert_eq!(target.len(), 16 * 9);
        assert!(target.chunks_exact(16).all(|row| {
            row[..2].iter().all(|pixel| *pixel == 0)
                && row[2..14].iter().all(|pixel| *pixel == 0x11_22_33)
                && row[14..].iter().all(|pixel| *pixel == 0)
        }));
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
    fn transitions_preserve_explored_view_and_end_at_destination() {
        for from in 0..3 {
            for to in 0..3 {
                if from == to {
                    continue;
                }
                let fill = 0.9;
                let rig = LightRig {
                    azimuth: 0.5,
                    elevation: 0.2,
                };
                let mut source = scene_with_charge(from, fill);
                rig.apply(&mut source);
                source.yaw = 0.65;
                let mut camera = default_camera(from);
                camera.az += 0.7;
                camera.distance += 1.;
                let mut transition = WorldTransition::from_view(from, to, camera, source.yaw);
                transition.moon_fill = fill;
                transition.from_light = source.lights[0].0;
                transition.to_light = rig.position(to);
                let mut expected = vec![0; 32 * 24];
                let mut output = expected.clone();
                let mut a = expected.clone();
                let mut b = expected.clone();
                render_world(&source, &camera, 32, 24, &mut expected).unwrap();
                render_transition(&transition, 32, 24, &mut output, &mut a, &mut b).unwrap();
                assert_eq!(
                    output, expected,
                    "start of {from} -> {to} must preserve the view"
                );
                transition.advance(800);
                let mut destination = scene_with_charge(to, fill);
                rig.apply(&mut destination);
                render_world(&destination, &default_camera(to), 32, 24, &mut expected).unwrap();
                render_transition(&transition, 32, 24, &mut output, &mut a, &mut b).unwrap();
                assert_eq!(
                    output, expected,
                    "end of {from} -> {to} must match destination"
                );
            }
        }
    }

    #[test]
    fn shadows_use_light_position() {
        let blocker = Cube {
            min: V::new(-0.5, -0.5, 2.),
            max: V::new(0.5, 0.5, 3.),
            material: mat(Kind::Stone),
            ship: false,
        };
        let mut cubes = vec![blocker];
        let bvh = build_bvh(&mut cubes);
        let scene = Scene {
            name: "test",
            cubes,
            bvh,
            spheres: vec![],
            lights: vec![(V::new(0., 0., 5.), V::new(1., 1., 1.))],
            yaw: 0.,
            sky: 0,
            moon_fill: 0.5,
        };
        assert!(!visible(&scene, V::new(0., 0., 0.), V::new(0., 0., 5.)));
        assert!(visible(&scene, V::new(2., 0., 0.), V::new(0., 0., 5.)));
    }

    #[test]
    fn accelerated_hits_and_shadows_match_linear_queries() {
        for id in 0..3 {
            let mut scene = scene(id);
            scene.yaw = 0.43;
            let camera = default_camera(id);
            for y in 0..24 {
                for x in 0..32 {
                    let ray = camera.ray(x, y, 32, 24);
                    let local = Ray {
                        o: ry(ray.o, -scene.yaw),
                        d: ry(ray.d, -scene.yaw),
                    };
                    let reference = scene
                        .cubes
                        .iter()
                        .filter_map(|&cube| hit_cube(cube, local))
                        .chain(
                            scene
                                .spheres
                                .iter()
                                .filter_map(|&sphere| hit_sphere(sphere, local)),
                        )
                        .min_by(|a, b| a.t.total_cmp(&b.t));
                    let accelerated = nearest(&scene, ray);
                    assert_eq!(accelerated.is_some(), reference.is_some());
                    if let (Some(a), Some(b)) = (accelerated, reference) {
                        assert!((a.t - b.t).abs() < EPS);
                        assert_eq!(a.material.kind, b.material.kind);
                        for &(light, _) in &scene.lights {
                            let delta = light - a.point;
                            let shadow = Ray {
                                o: a.point + delta.norm() * EPS * 4.,
                                d: delta.norm(),
                            };
                            assert_eq!(
                                visible(&scene, a.point, light),
                                nearest(&scene, shadow).is_none_or(|h| h.t > delta.len())
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn material_inspection_distinguishes_selection_from_context() {
        let scene = scene(0);
        assert!(scene_materials(&scene).contains(&Kind::StuccoTeal));
        let camera = default_camera(0);
        let mut sand = vec![0; 64 * 48];
        let mut metal = sand.clone();
        render_frame_mode(&scene, &camera, 64, 48, 1., &mut sand, Some(Kind::Sand)).unwrap();
        render_frame_mode(&scene, &camera, 64, 48, 1., &mut metal, Some(Kind::Metal)).unwrap();
        assert_ne!(framebuffer_hash(&sand), framebuffer_hash(&metal));
        assert!(inspection_title(&scene, Some(Kind::Water)).contains("transparencia=0.62"));
    }

    #[test]
    fn material_cycle_starts_without_m_and_hud_displays_selection() {
        let scene = scene(0);
        let kinds = scene_materials(&scene);
        let mut selected = None;
        for &kind in &kinds {
            selected = next_material(&scene, selected);
            assert_eq!(selected, Some(kind));
        }
        assert_eq!(next_material(&scene, selected), Some(kinds[0]));
        let lines = hud_lines(
            &scene,
            Some(Kind::Water),
            &MoonCharge::new(10),
            LightRig::default(),
            None,
        );
        assert!(lines
            .iter()
            .any(|line| line.contains("water") && line.contains("MATERIAL")));
        let mut pixels = vec![0; 640 * 480];
        draw_hud(&mut pixels, 640, 480, &lines);
        assert!(pixels.contains(&0xff_e1_98));
    }

    #[test]
    fn moon_charge_animates_continuously_with_bounded_voxel_size() {
        let mut charge = MoonCharge::new(10);
        charge.change(20);
        assert_eq!(charge.count, 20);
        assert_eq!(charge.fill, 0.5);
        charge.update(0.25);
        assert!(charge.fill > 0.5 && charge.fill < 1.);
        charge.update(0.25);
        assert_eq!(charge.fill, 1.);
        charge.change(-100);
        charge.update(0.5);
        assert_eq!(charge.fill, 0.);
        assert!(!charge.update(1.));
        let small = scene_with_charge(0, 0.);
        let full = scene_with_charge(0, 1.);
        assert_eq!(small.cubes.len(), full.cubes.len());
        let top = |scene: &Scene| {
            scene
                .cubes
                .iter()
                .filter(|c| c.ship)
                .map(|c| c.max.y)
                .fold(0., f32::max)
        };
        assert!(top(&full) > top(&small) + 1.5);
    }

    #[test]
    fn moving_key_light_changes_frame_and_preserves_fill_lights() {
        let mut scene = scene(0);
        let fills = scene.lights[1..].to_vec();
        let cam = default_camera(0);
        let mut before = vec![0; 64 * 48];
        let mut after = before.clone();
        render_world(&scene, &cam, 64, 48, &mut before).unwrap();
        LightRig {
            azimuth: 1.,
            elevation: -0.25,
        }
        .apply(&mut scene);
        render_world(&scene, &cam, 64, 48, &mut after).unwrap();
        assert_ne!(framebuffer_hash(&before), framebuffer_hash(&after));
        assert_eq!(scene.lights[1..], fills);
    }

    #[test]
    fn fresnel_and_total_internal_reflection_are_present() {
        assert!((schlick(1., 1., 1.5) - 0.04).abs() < 0.001);
        assert!(refract_direction(V::new(0., -1., 0.), V::new(0., 1., 0.), 1., 1.5).is_some());
        assert!(refract_direction(V::new(0.95, 0.312, 0.), V::new(0., 1., 0.), 1.5, 1.).is_none());
    }
}
