//! Reference-driven scenes, built locally from textured AABB blocks, not meshes.
use super::{cube, Cube, Kind, V};

pub const GALAXY_EXIT: V = V::new(3.4, 3.2, 3.0);
pub const CASTLE_EXIT: V = V::new(4.8, 1.6, 3.8);

fn ellipsoid(out: &mut Vec<Cube>, center: V, radii: V, cell: f32, material: Kind) {
    let nx = (radii.x / cell).ceil() as i32;
    let ny = (radii.y / cell).ceil() as i32;
    let nz = (radii.z / cell).ceil() as i32;
    let inside = |x: i32, y: i32, z: i32| {
        (x as f32 * cell / radii.x).powi(2)
            + (y as f32 * cell / radii.y).powi(2)
            + (z as f32 * cell / radii.z).powi(2)
            <= 1.
    };
    for x in -nx..=nx {
        for y in -ny..=ny {
            for z in -nz..=nz {
                // A closed voxel shell removes invisible interior blocks.
                if inside(x, y, z)
                    && [
                        (1, 0, 0),
                        (-1, 0, 0),
                        (0, 1, 0),
                        (0, -1, 0),
                        (0, 0, 1),
                        (0, 0, -1),
                    ]
                    .iter()
                    .any(|(dx, dy, dz)| !inside(x + dx, y + dy, z + dz))
                {
                    cube(
                        out,
                        center + V::new(x as f32, y as f32, z as f32) * cell,
                        V::new(cell, cell, cell),
                        material,
                    );
                }
            }
        }
    }
}

fn star(out: &mut Vec<Cube>, center: V, cell: f32) {
    for (row, mask) in [
        0b1000001u8,
        0b1100011,
        0b0111110,
        0b0111110,
        0b1111111,
        0b0011100,
        0b0001000,
    ]
    .iter()
    .enumerate()
    {
        for x in 0..7 {
            if mask & (1 << x) != 0 {
                cube(
                    out,
                    center + V::new(x as f32 - 3., row as f32 - 3., 0.) * cell,
                    V::new(cell, cell, cell * 1.2),
                    Kind::Star,
                );
            }
        }
    }
}

fn tree(out: &mut Vec<Cube>, x: f32, y: f32, z: f32, scale: f32) {
    cube(
        out,
        V::new(x, y + 0.35 * scale, z),
        V::new(0.18, 0.70, 0.18) * scale,
        Kind::Bark,
    );
    for i in 0..4 {
        let width = (0.9 - i as f32 * 0.19) * scale;
        cube(
            out,
            V::new(x, y + (0.7 + i as f32 * 0.24) * scale, z),
            V::new(width, 0.30 * scale, width),
            Kind::Grass,
        );
    }
}

fn roof(out: &mut Vec<Cube>, center: V, width: f32, depth: f32, height: f32) {
    let layers = (height / 0.22).ceil() as usize;
    for i in 0..layers {
        let t = i as f32 / layers as f32;
        cube(
            out,
            center + V::new(0., i as f32 * 0.22, 0.),
            V::new(
                (width * (1. - t)).max(0.18),
                0.23,
                (depth * (1. - t)).max(0.18),
            ),
            Kind::Roof,
        );
    }
}

fn pipe(out: &mut Vec<Cube>, center: V) {
    cube(
        out,
        center - V::new(0., 0.70, 0.),
        V::new(0.9, 1.4, 0.9),
        Kind::Pipe,
    );
    for dx in [-0.48, 0.48] {
        cube(
            out,
            center + V::new(dx, 0., 0.),
            V::new(0.30, 0.28, 1.26),
            Kind::Pipe,
        );
    }
    for dz in [-0.48, 0.48] {
        cube(
            out,
            center + V::new(0., 0., dz),
            V::new(0.66, 0.28, 0.30),
            Kind::Pipe,
        );
    }
    cube(
        out,
        center - V::new(0., 0.12, 0.),
        V::new(0.65, 0.05, 0.65),
        Kind::Dark,
    );
}

pub fn galaxy(out: &mut Vec<Cube>) {
    // Mario_Galaxy2.png: beige head-planetoid, planted green cap, projecting nose.
    ellipsoid(
        out,
        V::new(0., 2.5, 0.),
        V::new(2.35, 2.10, 2.1),
        0.34,
        Kind::Skin,
    );
    for x in [-2.25, 2.25] {
        ellipsoid(
            out,
            V::new(x, 2.65, 0.05),
            V::new(0.50, 0.72, 0.50),
            0.23,
            Kind::Skin,
        );
        ellipsoid(
            out,
            V::new(x, 2.65, 0.43),
            V::new(0.24, 0.38, 0.18),
            0.16,
            Kind::Path,
        );
    }
    ellipsoid(
        out,
        V::new(0., 2.55, 2.05),
        V::new(0.76, 0.65, 0.92),
        0.24,
        Kind::Skin,
    );
    for x in [-0.68, 0.68] {
        cube(
            out,
            V::new(x, 3.28, 1.97),
            V::new(0.42, 0.64, 0.18),
            Kind::Castle,
        );
        cube(
            out,
            V::new(x + 0.04, 3.26, 2.08),
            V::new(0.16, 0.38, 0.09),
            Kind::Dark,
        );
        cube(
            out,
            V::new(x, 3.73, 1.81),
            V::new(0.56, 0.18, 0.25),
            Kind::Planet,
        );
    }
    // Green moustache and sideburns, matching the garden-planet reference.
    for x in -3i32..=3 {
        cube(
            out,
            V::new(x as f32 * 0.32, 1.86 - (x.abs() % 2) as f32 * 0.12, 1.96),
            V::new(0.34, 0.40, 0.34),
            Kind::Planet,
        );
    }
    for x in [-1.78, 1.78] {
        cube(
            out,
            V::new(x, 3.05, 1.08),
            V::new(0.35, 0.95, 0.45),
            Kind::Planet,
        );
    }
    // Cap crown is a broad voxel ellipsoid rather than a smooth sphere.
    ellipsoid(
        out,
        V::new(0., 4.35, -0.15),
        V::new(2.38, 1.10, 2.10),
        0.32,
        Kind::Grass,
    );
    for x in -6..=6 {
        for z in -5..=5 {
            if (x as f32 / 6.).powi(2) + (z as f32 / 5.).powi(2) <= 1. {
                cube(
                    out,
                    V::new(x as f32 * 0.36, 4.13, z as f32 * 0.36),
                    V::new(0.36, 0.22, 0.36),
                    Kind::Castle,
                );
            }
        }
    }
    for x in -5i32..=5 {
        cube(
            out,
            V::new(x as f32 * 0.34, 4.03, 2.0),
            V::new(0.34, 0.24, 1.5),
            Kind::Grass,
        );
        cube(
            out,
            V::new(x as f32 * 0.34, 3.90, 2.62),
            V::new(0.34, 0.10, 0.28),
            Kind::Castle,
        );
    }
    // Cap badge with a hand-built red M glyph.
    cube(
        out,
        V::new(0., 4.62, 1.84),
        V::new(1.06, 0.94, 0.24),
        Kind::Castle,
    );
    for (row, mask) in [0b10001u8, 0b11011, 0b10101, 0b10001, 0b10001]
        .iter()
        .enumerate()
    {
        for x in 0..5 {
            if mask & (1 << x) != 0 {
                cube(
                    out,
                    V::new((x as f32 - 2.) * 0.15, 4.93 - row as f32 * 0.15, 1.985),
                    V::new(0.15, 0.15, 0.08),
                    Kind::Roof,
                );
            }
        }
    }
    for (x, z) in [(-1.2, -0.7), (1.15, -0.7), (-0.9, 0.4)] {
        tree(out, x, 5.0, z, 0.62);
    }
    cube(
        out,
        V::new(0.35, 5.27, -0.80),
        V::new(0.65, 0.50, 0.65),
        Kind::Castle,
    );
    roof(out, V::new(0.35, 5.59, -0.80), 0.85, 0.85, 0.5);
    for i in 0..8 {
        cube(
            out,
            V::new(-0.3 + i as f32 * 0.10, 5.35, -0.05 - i as f32 * 0.10),
            V::new(0.19, 0.07, 0.19),
            Kind::Path,
        );
    }
    star(out, GALAXY_EXIT, 0.22);
    for i in 0..14 {
        let a = i as f32 * std::f32::consts::TAU / 14.;
        let p = V::new(4.7 * a.cos(), 2.8 + 1.25 * a.sin(), 3.0 * a.sin() - 0.6);
        if i % 3 == 0 {
            star(out, p, 0.075);
        } else {
            cube(out, p, V::new(0.13, 0.13, 0.13), Kind::Star);
        }
    }
}

pub fn mario64(out: &mut Vec<Cube>) {
    // Mario64.png: a bounded square garden with moat, lake and Peach's castle.
    for x in -7i32..=7 {
        for z in -6..=6 {
            let xf = x as f32;
            let zf = z as f32;
            let moat = x.abs() <= 5 && (-4..=1).contains(&z);
            let lake = x >= 3 && z >= 2;
            cube(
                out,
                V::new(xf, -0.55, zf),
                V::new(0.995, 0.8, 0.995),
                Kind::Stone,
            );
            if moat || lake {
                cube(
                    out,
                    V::new(xf, -0.125, zf),
                    V::new(0.995, 0.04, 0.995),
                    Kind::LakeBed,
                );
            }
            cube(
                out,
                V::new(xf, 0.035, zf),
                V::new(0.995, 0.12, 0.995),
                if moat || lake {
                    Kind::LakeWater
                } else if z >= 2 && ((xf + (zf * 0.5).sin() * 2.0).abs() < 1.4 || z == 5) {
                    Kind::Path
                } else {
                    Kind::Grass
                },
            );
        }
    }
    for (x, z, height) in [
        (-6., -4., 2.0),
        (-6., -2., 1.3),
        (6., -4., 1.6),
        (5.8, -5., 1.0),
    ] {
        for layer in 0..5 {
            let size = 2.6 - layer as f32 * 0.43;
            cube(
                out,
                V::new(x, 0.15 + layer as f32 * height / 5., z),
                V::new(size, height / 5., size),
                Kind::Grass,
            );
        }
    }
    // Textured stone wall courses; individual blocks remain visible all around.
    for x in -5i32..=5 {
        for z in -3..=2 {
            for y in 0..5 {
                if x.abs() < 5 && (-2..=1).contains(&z) && (1..4).contains(&y) {
                    continue;
                }
                cube(
                    out,
                    V::new(
                        x as f32 * 0.58,
                        0.62 + y as f32 * 0.58,
                        -2.3 + z as f32 * 0.58,
                    ),
                    V::new(0.575, 0.575, 0.575),
                    Kind::Castle,
                );
            }
        }
    }
    roof(out, V::new(0., 3.63, -2.58), 7.1, 4.7, 1.75);
    for x in [-3.1, 3.1] {
        for z in [-3.5, -0.95] {
            for layer in 0..7 {
                cube(
                    out,
                    V::new(x, 0.60 + layer as f32 * 0.55, z),
                    V::new(1.40, 0.545, 1.4),
                    Kind::Castle,
                );
            }
            roof(out, V::new(x, 4.52, z), 2.03, 2.03, 1.6);
            cube(
                out,
                V::new(x, 3.20, z + 0.71),
                V::new(0.35, 0.7, 0.07),
                Kind::Dark,
            );
        }
    }
    // Tall central keep and tiered red spire with a white flag.
    for layer in 0..8 {
        cube(
            out,
            V::new(0., 4.33 + layer as f32 * 0.46, -2.4),
            V::new(1.43, 0.455, 1.43),
            Kind::Castle,
        );
    }
    roof(out, V::new(0., 8.05, -2.4), 2.0, 2.0, 1.45);
    cube(
        out,
        V::new(0., 9.75, -2.4),
        V::new(0.06, 0.9, 0.06),
        Kind::Metal,
    );
    cube(
        out,
        V::new(0.27, 10.0, -2.4),
        V::new(0.50, 0.30, 0.06),
        Kind::Castle,
    );
    cube(
        out,
        V::new(0., 6.92, -1.67),
        V::new(0.37, 0.72, 0.08),
        Kind::Dark,
    );
    // Main arched entry and its Peach-coloured medallion above.
    cube(
        out,
        V::new(0., 1.20, -0.52),
        V::new(1.23, 1.70, 0.20),
        Kind::Bark,
    );
    cube(
        out,
        V::new(0., 2.05, -0.48),
        V::new(0.80, 0.35, 0.20),
        Kind::Dark,
    );
    for x in [-0.78, 0.78] {
        cube(
            out,
            V::new(x, 1.25, -0.42),
            V::new(0.24, 2.0, 0.30),
            Kind::Path,
        );
    }
    cube(
        out,
        V::new(0., 2.23, -0.41),
        V::new(1.45, 0.21, 0.32),
        Kind::Path,
    );
    cube(
        out,
        V::new(0., 2.89, -0.37),
        V::new(0.94, 1.00, 0.23),
        Kind::Path,
    );
    cube(
        out,
        V::new(0., 2.93, -0.21),
        V::new(0.51, 0.63, 0.07),
        Kind::StuccoMagenta,
    );
    cube(
        out,
        V::new(0., 3.04, -0.15),
        V::new(0.26, 0.34, 0.07),
        Kind::Skin,
    );
    cube(
        out,
        V::new(0., 3.33, -0.15),
        V::new(0.46, 0.12, 0.08),
        Kind::Star,
    );
    for x in [-2.2, -1.4, 1.4, 2.2] {
        for y in [1.28, 2.58] {
            cube(
                out,
                V::new(x, y, -0.53),
                V::new(0.43, 0.55, 0.16),
                Kind::Dark,
            );
            cube(
                out,
                V::new(x, y, -0.41),
                V::new(0.065, 0.54, 0.08),
                Kind::Castle,
            );
        }
    }
    for step in 0..8 {
        let z = 2.8 - step as f32 * 0.43;
        cube(
            out,
            V::new(0., 0.14 + step as f32 * 0.025, z),
            V::new(1.65, 0.20, 0.44),
            Kind::Path,
        );
        for x in [-0.96, 0.96] {
            cube(
                out,
                V::new(x, 0.35, z),
                V::new(0.16, 0.48, 0.42),
                Kind::Castle,
            );
        }
    }
    for (x, z) in [
        (-5.8, 4.8),
        (-5.0, 2.8),
        (-4.8, 0.8),
        (-3.8, 4.4),
        (-2.2, 3.7),
        (-2.5, 5.5),
        (1.6, 5.4),
        (2.0, 3.6),
        (6.0, 0.9),
        (5.8, -1.5),
        (4.8, -5.2),
        (-4.5, -5.2),
    ] {
        tree(out, x, 0.1, z, 0.80);
    }
    pipe(out, CASTLE_EXIT);
    star(out, V::new(-3.5, 1.8, 2.8), 0.12);
    for x in [-1.5, -0.8, 0.0] {
        cube(
            out,
            V::new(x, 0.48, 4.8),
            V::new(0.24, 0.45, 0.12),
            Kind::Star,
        );
    }
}
