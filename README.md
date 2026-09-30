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
| `N` | Transición al siguiente mundo: Odyssey → Galaxy → NSMB Wii → Odyssey |
| `1` / `2` / `3` | Transición directa a Odyssey / Galaxy / NSMB Wii |
| `Esc` o cerrar la ventana | Salir |

Para exportar explícitamente un PPM sin ventana:

```bash
cargo run -- --headless --scene 0 --width 320 --height 240 --output odyssey.ppm
# --render es un alias de --headless
```

Use una ruta terminada en `.png` para exportar PNG; cualquier otra extensión conserva PPM.

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
delta-time para movimiento continuo. `WorldTransition` conserva explícitamente mundo
origen/destino, progreso acotado `0..1`, cámara interpolada y duración fija de **0.8 s**.
Cada frame renderiza ambos mundos, aplica easing smoothstep y compone sus framebuffers con
crossfade y un fundido a negro leve; no hay salto instantáneo. Mientras una transición está
activa, `N` y `1/2/3` se ignoran de forma determinista (no se encolan ni reinician); al
terminar, el mundo destino queda activo. Esc sigue cerrando la ventana y no se genera
`render.ppm` por defecto.
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

La pasada visual v3 en release a 320×240 midió 235.29 FPS (Odyssey), 285.71 FPS
(Galaxy) y 17.12 FPS (NSMB Wii), con 11 de 11 cambios de framebuffer durante la
órbita simulada. Odyssey→Galaxy durante la transición midió 107.46 FPS. El benchmark
mide render CPU y movimiento de cámara; la ventana requiere un display X11/Wayland.

Las pruebas unitarias cubren intersección slab, existencia de geometría/materiales y órbita de cámara. El smoke test de exportación headless puede ejecutarse sin display con una resolución pequeña y una ruta temporal:

```bash
tmpdir=$(mktemp -d)
cargo run -- --headless --width 32 --height 24 --output "$tmpdir/smoke.ppm"
test -s "$tmpdir/smoke.ppm"
```

Para validar la transición sin display se puede generar una secuencia pequeña de diez
PNG (inicio, tres puntos intermedios y final para Odyssey→Galaxy y Galaxy→NSMB Wii) junto
con hashes, tiempos de render y FPS:

```bash
cargo run --release -- --transition-demo artifacts/transition-demo --width 96 --height 72
```

El manifest queda en `artifacts/transition-demo/manifest.json`; los hashes de los frames
intermedios deben ser distintos y el último frame declara `NSMB Wii` como mundo final.
Para medir rendimiento, `--benchmark` imprime tanto el FPS de órbita continua como
`TRANSITION_BENCHMARK` para el crossfade. En el benchmark release de 320×240 se exige
mantener al menos 10 FPS en órbita; la transición mide por separado sus dos renders por
frame. No se declara una ventana real validada cuando no hay display/Xvfb disponible.

Las capturas y el GIF existentes en `artifacts/` son material de entrega; no son generados automáticamente por `cargo run`.

## Estado visual y límites

La revisión visual v3 usa cámaras 3/4 específicas, iluminación key/fill/rim, tone
mapping y composiciones separadas: Odyssey flota contra el skybox con casco rojo curvo,
copa, ala clara, globo, barandas y ojo de buey refractivo; Galaxy combina océano,
continentes, accidentes y órbita inclinada; NSMB Wii muestra puerta, almenas, banderas,
tuberías, bloques y monedas. Los PNG medidos, hashes y comparación con v2 están en
`artifacts/review-v3/`; la nueva secuencia está en `artifacts/transition-demo-v3/`.

El dip negro de transición está limitado a 7 %, para que el frame medio siga mostrando
los dos mundos. La calidad sigue siendo procedural y estilizada: no hay modelos,
texturas pintadas, antialiasing ni bloom, y no se validó una ventana real sin display/Xvfb.

La estética sigue siendo estilizada y procedural: no hay modelos ni texturas pintadas a
mano, bloom, antialiasing, ni assets de personajes. Es una mejora de legibilidad y
composición, no una reproducción exacta de los juegos.
