//! Prepared scenes and adaptive-resolution CPU travel, without frame playback.
use super::*;

pub struct Renderer {
    original: Scene,
    source: Scene,
    destination: Scene,
    baseline: Vec<Cube>,
    from_buffer: Vec<u32>,
    to_buffer: Vec<u32>,
    blended: Vec<u32>,
}

/// Full quality at both endpoints; half width/height through the moving middle.
pub fn resolution(w: u32, h: u32, progress: f32) -> (u32, u32) {
    let edge = (progress.min(1. - progress) / 0.08).clamp(0., 1.);
    let scale = 1. - 0.5 * smoothstep(edge);
    (
        (w as f32 * scale).round().max(1.) as u32,
        (h as f32 * scale).round().max(1.) as u32,
    )
}

fn refit(scene: &mut Scene) {
    // Children follow their parents in the flattened BVH; visit in reverse.
    for i in (0..scene.bvh.len()).rev() {
        let node = &scene.bvh[i];
        let (min, max) = if let Some((a, b)) = node.children {
            let (a, b) = (&scene.bvh[a], &scene.bvh[b]);
            (
                V::new(
                    a.min.x.min(b.min.x),
                    a.min.y.min(b.min.y),
                    a.min.z.min(b.min.z),
                ),
                V::new(
                    a.max.x.max(b.max.x),
                    a.max.y.max(b.max.y),
                    a.max.z.max(b.max.z),
                ),
            )
        } else {
            let mut min = V::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
            let mut max = -min;
            for cube in &scene.cubes[node.start..node.start + node.count] {
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
            (min, max)
        };
        scene.bvh[i].min = min;
        scene.bvh[i].max = max;
    }
}

impl Renderer {
    pub fn into_destination(self) -> Scene {
        self.destination
    }

    pub fn new(transition: &WorldTransition) -> Self {
        let mut original = scene_with_charge(transition.from, transition.moon_fill);
        original.lights[0].0 = transition.from_light;
        original.yaw = transition.from_yaw;
        let mut source = original.clone();
        let mut destination = scene_with_charge(transition.to, transition.moon_fill);
        destination.lights[0].0 = transition.to_light;
        if transition.from == 0 {
            // Reserve exhaust primitives and build this topology just once.
            for x in [-1.22, 1.22] {
                cube(
                    &mut source.cubes,
                    V::new(x, 0.35, -2.62),
                    V::new(0.3, EPS, 0.3),
                    Kind::Star,
                );
            }
            source.bvh = build_bvh(&mut source.cubes);
        }
        let baseline = source.cubes.clone();
        Self {
            original,
            source,
            destination,
            baseline,
            from_buffer: Vec::new(),
            to_buffer: Vec::new(),
            blended: Vec::new(),
        }
    }

    pub fn render(
        &mut self,
        transition: &WorldTransition,
        w: u32,
        h: u32,
        output: &mut [u32],
    ) -> io::Result<u128> {
        let started = Instant::now();
        let t = transition.eased_progress();
        // Invisible worlds need no rays. Endpoints remain pixel-exact normal renders.
        if t == 0. {
            render_world(&self.original, &transition.from_camera, w, h, output)?;
            return Ok(started.elapsed().as_millis());
        }
        if t == 1. {
            render_world(
                &self.destination,
                &default_camera(transition.to),
                w,
                h,
                output,
            )?;
            return Ok(started.elapsed().as_millis());
        }
        self.source.yaw = transition.from_yaw * (1. - t);
        let lift = if transition.from == 0 { 6. * t * t } else { 0. };
        for (cube, base) in self.source.cubes.iter_mut().zip(&self.baseline) {
            *cube = *base;
            if transition.from == 0 {
                if cube.ship {
                    cube.min.y += lift;
                    cube.max.y += lift;
                } else if cube.material.kind == Kind::Star {
                    // Odyssey's two reserved exhaust blocks; desert has no Star blocks.
                    let center = V::new(
                        (base.min.x + base.max.x) * 0.5,
                        lift + 0.35 - t * 0.5,
                        -2.62,
                    );
                    let half = V::new(0.15, t * 0.75, 0.15);
                    cube.min = center - half;
                    cube.max = center + half;
                }
            } else if transition.from == 1 && cube.material.kind == Kind::Star {
                cube.material.emission = base.material.emission * (1. + 8. * t * (1. - t));
            }
        }
        if transition.from == 0 {
            refit(&mut self.source);
        }
        let (anchor, distance, elevation) = match transition.from {
            0 => (V::new(0., 3.4 + lift, 0.), 4.5, 0.35),
            1 => (worlds::GALAXY_EXIT, 0.85, 0.25),
            _ => (worlds::CASTLE_EXIT, 0.60, 1.35),
        };
        let from = transition.from_camera;
        let source_camera = Camera {
            target: from.target.lerp(ry(anchor, self.source.yaw), t),
            distance: from.distance * (1. - t) + distance * t,
            az: from.az,
            el: from.el * (1. - t) + elevation * t,
        };
        let to = default_camera(transition.to);
        let entry_camera = Camera {
            target: to.target + V::new(0., 1.2 * (1. - t), 0.),
            distance: to.distance + 4. * (1. - t),
            az: to.az + 0.25 * (1. - t),
            el: to.el + 0.25 * (1. - t),
        };
        let (rw, rh) = resolution(w, h, transition.progress());
        let size = (rw * rh) as usize;
        self.from_buffer.resize(size, 0);
        self.to_buffer.resize(size, 0);
        self.blended.resize(size, 0);
        render_world(&self.source, &source_camera, rw, rh, &mut self.from_buffer)?;
        render_world(
            &self.destination,
            &entry_camera,
            rw,
            rh,
            &mut self.to_buffer,
        )?;
        composite_transition(&self.from_buffer, &self.to_buffer, t, &mut self.blended);
        for y in 0..h as usize {
            for x in 0..w as usize {
                output[y * w as usize + x] = self.blended
                    [(y * rh as usize / h as usize) * rw as usize + x * rw as usize / w as usize];
            }
        }
        Ok(started.elapsed().as_millis())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adaptive_resolution_is_bounded_and_recovers_endpoint_quality() {
        for (w, h) in [(320, 240), (33, 25), (1, 1)] {
            assert_eq!(resolution(w, h, 0.), (w, h));
            assert_eq!(resolution(w, h, 1.), (w, h));
            for n in 0..=100 {
                let (rw, rh) = resolution(w, h, n as f32 / 100.);
                assert!((1..=w).contains(&rw) && (1..=h).contains(&rh));
            }
        }
        assert_eq!(resolution(320, 240, 0.5), (160, 120));
    }
    #[test]
    fn prepared_animation_does_not_accumulate_and_refit_matches_rebuild() {
        for from in 0..3 {
            let mut state = WorldTransition::new(from, (from + 1) % 3);
            let mut cached = Renderer::new(&state);
            let counts = (cached.source.cubes.len(), cached.source.bvh.len());
            let mut output = vec![0; 32 * 24];
            state.elapsed_ms = 400;
            cached.render(&state, 32, 24, &mut output).unwrap();
            let expected = output.clone();
            state.elapsed_ms = 600;
            cached.render(&state, 32, 24, &mut output).unwrap();
            state.elapsed_ms = 400;
            cached.render(&state, 32, 24, &mut output).unwrap();
            assert_eq!(output, expected);
            assert_eq!(counts, (cached.source.cubes.len(), cached.source.bvh.len()));
            let mut rebuilt = cached.source.clone();
            rebuilt.bvh = build_bvh(&mut rebuilt.cubes);
            let mut a = vec![0; 48 * 36];
            let mut b = a.clone();
            render_world(&cached.source, &default_camera(from), 48, 36, &mut a).unwrap();
            render_world(&rebuilt, &default_camera(from), 48, 36, &mut b).unwrap();
            assert_eq!(a, b);
        }
    }
}
