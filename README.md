# Proyecto 2 — Dioramas con raytracing

Renderer de CPU en Rust para tres dioramas: Mario Odyssey, Mario Galaxy y NSMB Wii. Conserva geometría AABB con UV por cara, texturas procedurales, sombras, reflexión, refracción, emisión y skyboxes por escena.

## Ejecutar

El comportamiento normal abre una ventana interactiva y la mantiene activa hasta cerrarla:

```bash
cargo run
```

El framebuffer se renderiza en paralelo por filas y se presenta después de cada frame. No se crea ningún `render.ppm` al ejecutar así.

Controles:

| Tecla | Acción |
| --- | --- |
| Flechas o `WASD` mantenidas | Orbitar y elevar/bajar la cámara continuamente |
| `+` / `-` mantenidas | Zoom continuo |
| `R` mantenida | Rotar el diorama continuamente |
| `N` | Cambiar de escena con transición fade |
| `Esc` o cerrar la ventana | Salir |

Para exportar explícitamente un PPM sin ventana:

```bash
cargo run -- --headless --scene 0 --width 320 --height 240 --output odyssey.ppm
# --render es un alias de --headless
```

El modo headless exige `--output`; así `cargo run` y un headless incompleto no crean
`render.ppm` accidentalmente. Para medir doce frames con órbita continua y comprobar
que el framebuffer cambia:

```bash
cargo run --release -- --benchmark --scene 0 --width 320 --height 240
```

`--scene` acepta `0` (Odyssey), `1` (Galaxy) o `2` (NSMB Wii). El modo headless es útil para smoke tests y exportación; los renders generados deben mantenerse fuera del working tree o ignorados.

## Requisitos

- Rust estable y Cargo.
- En Linux, un servidor X11/Wayland para `cargo run`; en CI sin display puede usarse `xvfb-run`.
- `minifb` proporciona la ventana y el blit del framebuffer. El raytracer permanece implementado en el proyecto, sin motor gráfico externo.

## Arquitectura adoptada

Se consultó directamente la rama pública [`18-RT-06-REFLECTIONS`](https://github.com/menene/cc2018-2026-02-10/tree/18-RT-06-REFLECTIONS) del repositorio de referencia. Se adaptó su patrón técnico de `minifb`: `Vec<u32>` como framebuffer, `Window::is_open`, polling de teclado, cámara orbital y `update_with_buffer`. La adaptación conserva la arquitectura y los materiales propios del Proyecto 2; no se incorporó su historial ni se copió su escena.

La ventana rerenderiza cuando cambia la cámara o la escena; las teclas mantenidas usan
delta-time para movimiento continuo. `N` presenta seis pasos de fade entre dioramas.
Rayon paraleliza las filas del framebuffer. Reflexión conserva la energía local/reflejada,
refracción usa Schlick y maneja reflexión interna total; sombras, texturas y skyboxes
siguen resolviéndose en `trace`.

## Verificación

```bash
cargo fmt --check
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
cargo run --release -- --benchmark --scene 0 --width 320 --height 240
```

En el equipo de desarrollo, el benchmark release a 320×240 midió 11.92 FPS
(Odyssey), 15.60 FPS (Galaxy) y 15.17 FPS (NSMB Wii), con 11 de 11 cambios de
framebuffer durante la órbita simulada. El tiempo de exportación observado fue de
60–76 ms por frame. El benchmark mide el render CPU y el movimiento de cámara; la
ventana requiere un display X11/Wayland funcional.

Las pruebas unitarias cubren intersección slab, existencia de geometría/materiales y órbita de cámara. El smoke test de exportación headless puede ejecutarse sin display con una resolución pequeña y una ruta temporal:

```bash
tmpdir=$(mktemp -d)
cargo run -- --headless --width 32 --height 24 --output "$tmpdir/smoke.ppm"
test -s "$tmpdir/smoke.ppm"
```

Las capturas y el GIF existentes en `artifacts/` son material de entrega; no son generados automáticamente por `cargo run`.

## Estado visual y límites

La segunda revisión visual usa cámaras 3/4 específicas, iluminación key/fill/rim,
ambiente mínimo, esferas raytraceadas y composiciones nuevas: Odyssey tiene casco rojo,
cubierta y globo; Galaxy un planetoide y órbita; NSMB Wii castillo, tuberías, bloques y
monedas. Los PNG medidos y sus métricas están en `artifacts/review-v2/`.

La estética sigue siendo estilizada y procedural: no hay modelos ni texturas pintadas a
mano, bloom, antialiasing, ni assets de personajes. Es una mejora de legibilidad y
composición, no una reproducción exacta de los juegos.
