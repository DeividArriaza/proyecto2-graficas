# Proyecto 2 — Diorama Mario (Raytracer en Rust)

**Fecha:** 2026-09-28
**Stack:** Rust + raytracer por software (CPU)
**Plazo:** 4 semanas
**Concepto:** Tres dioramas conectados en ciclo — Mario Odyssey → Mario Galaxy → New Super Mario Bros. Wii → Mario Odyssey

---

## 1. Concepto

Tres escenas raytraceadas, cada una con identidad visual propia, unidas por transiciones cinemáticas que forman un ciclo cerrado:

| Origen | Vehículo de transición | Destino |
|---|---|---|
| **Odyssey** | Capi / la nave Odyssey despega | **Galaxy** |
| **Galaxy** | Launch Star (estrella de lanzamiento) | **NSMB Wii** |
| **NSMB Wii** | Tubería verde (el personaje entra al túnel) | **Odyssey** |

No hay gameplay. El usuario orbita la cámara libremente dentro de un diorama y presiona una tecla para disparar la transición al siguiente.

---

## 2. Veredicto de viabilidad

**Viable en 4 semanas**, con dos condiciones no negociables:

1. **Toda la geometría se construye con cubos (AABB) texturizados**, estilo voxel.
2. **Sin gameplay, sin física, sin animación esquelética.**

### Por qué los cubos salvan el proyecto

- La intersección rayo-AABB (*slab method*) es de ~10 operaciones. La intersección rayo-triángulo con BVH profundo es órdenes de magnitud más cara.
- **Elimina por completo el problema de assets.** No hay Blender, ni conversión de formatos propietarios de Switch/Wii (`BFRES`, `SARC`, `BRRES`), ni UVs rotos. Un diorama es un array de `(x, y, z, material)`.
- Estilo coherente entre las tres escenas.
- Los mundos se reconocen por **color, skybox, silueta y materiales**, no por conteo de polígonos.

> Si se intentara usar mallas rippeadas de los juegos reales (50k+ triángulos por modelo × 3 escenas), el render tomaría segundos por frame y el proyecto sería inviable en el plazo.

---

## 3. Riesgos ordenados por dolor real

### 3.1 Rendimiento — es EL problema

Sin resolver esto, no hay transiciones, ni cámara, ni proyecto. Se necesitan las cuatro medidas juntas:

- **`rayon`** → `par_chunks_mut` sobre el framebuffer. Ganancia casi lineal por core.
- **BVH o grid uniforme** por escena. Sin estructura de aceleración, 3000 cubos × 120k píxeles es inviable.
- **Resolución adaptativa**: renderizar a 1/2 o 1/4 mientras la cámara se mueve, full res al detenerse. Truco barato, impacto enorme.
- **Profundidad de recursión ≤ 3**, y cortar el rayo cuando su contribución cae bajo un umbral (~0.01).

**Estimado realista** (build release, 8 cores, ~3000 cubos, BVH):

| Resolución | FPS esperado |
|---|---|
| 800×600 | ~4-10 FPS |
| 400×300 escalado ×2 | ~20-40 FPS |

Trabajar a baja resolución interna y escalar el buffer a la ventana.

### 3.2 Compilar siempre en `--release`

Bug #1 clásico de este proyecto. El modo debug de Rust es **30-70× más lento** en código numérico. Perfilar o juzgar el rendimiento en debug lleva a conclusiones falsas y a "optimizaciones" innecesarias.

```bash
cargo run --release
```

Además, en `Cargo.toml`:

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
```

### 3.3 Transiciones cinemáticas

Cada frame de la transición se raytrace completo. Tres estrategias, de mejor a peor:

1. **Bajar resolución agresivamente durante la transición** (1/4 res + fade). El movimiento de cámara tapa la pérdida de detalle. **Recomendado.**
2. Transiciones cortas (30-45 frames) con fade a negro en el punto medio, donde ocurre el swap de escena.
3. Pre-renderizar la secuencia a PNGs y reproducirla. Funciona, pero es más difícil de defender ante el catedrático.

### 3.4 Materiales — es donde está la nota

Estos cursos suelen puntuar reflexión, refracción/transparencia, Fresnel, emisión y texturas. Repartirlos por escena para que cada diorama justifique su existencia:

| Diorama | Materiales que luce | Skybox |
|---|---|---|
| **Odyssey** | Agua **refractiva** (Fresnel + Snell), metal **reflectivo** de la ciudad, texturas difusas | Cielo urbano / atardecer |
| **Galaxy** | **Emisión** (estrellas, Launch Star), planetoides, bloom fake | Espacio estrellado |
| **NSMB Wii** | Difusos saturados, ladrillo texturizado, tubería verde, luz plana | Cielo azul con nubes |

### 3.5 Texturas por cara de cubo

Hay que mapear UV según qué cara del AABB golpeó el rayo (comparando el punto de impacto contra los planos del cubo). No es difícil, pero es la fuente clásica de texturas espejadas o rotadas 90°.

### 3.6 Skybox por escena

Cambiar el cubemap al cambiar de diorama es barato y **es lo que más va a vender que son tres mundos distintos**. Prioridad alta, costo bajo.

---

## 4. Lo que NO es un problema

Gracias a la decisión de voxels, quedan eliminados del alcance:

- Assets 3D y conversión de formatos
- Animación esquelética
- Colisiones y física
- Gravedad esférica en Galaxy (es **solo visual**; intentar física custom ahí cuesta una semana)

---

## 5. Arquitectura propuesta

```
proyecto2-graficas/
├── Cargo.toml
├── PLAN.md
├── assets/
│   ├── textures/          # PNGs de ladrillo, tubería, metal, agua, estrella...
│   └── skyboxes/          # 3 cubemaps (o equirectangulares)
└── src/
    ├── main.rs            # loop principal, input, máquina de estados
    ├── framebuffer.rs     # buffer interno + upscale a ventana
    ├── vector.rs          # Vec3 propio (si el curso lo exige) o glam
    ├── ray.rs
    ├── camera.rs          # orbital + modo cinemático (spline)
    ├── material.rs        # difuso, reflectivo, refractivo, emisivo
    ├── texture.rs         # carga + sampleo por cara de cubo
    ├── cube.rs            # AABB + slab intersection + UV por cara
    ├── bvh.rs             # estructura de aceleración
    ├── light.rs
    ├── skybox.rs
    ├── transition.rs      # spline de cámara + fade + swap de escena
    └── scene/
        ├── mod.rs         # trait Scene + registro de escenas
        ├── odyssey.rs
        ├── galaxy.rs
        └── wii.rs
```

### Crates

```toml
[dependencies]
rayon = "1"          # paralelismo del render
image = "0.25"       # carga de texturas PNG
glam = "0.29"        # vectores/matrices (omitir si el curso exige implementarlos)
raylib = "5"         # ventana + input   (alternativa: minifb = "0.27")
```

> Si el curso exige implementar la matemática vectorial a mano, quitar `glam` y usar `src/vector.rs`.

### Diseño clave: `trait Scene` genérico

El sistema debe soportar **N escenas** desde el día 1, no exactamente 3. Así, recortar a 2 dioramas no requiere reescribir nada.

```rust
pub trait Scene {
    fn name(&self) -> &str;
    fn objects(&self) -> &Bvh;
    fn lights(&self) -> &[Light];
    fn skybox(&self) -> &Skybox;
    fn default_camera(&self) -> Camera;
    /// Punto de salida de la cámara al iniciar la transición
    fn exit_anchor(&self) -> Vec3;
    /// Punto de entrada de la cámara al llegar desde otra escena
    fn entry_anchor(&self) -> Vec3;
}
```

### Máquina de estados

```rust
enum AppState {
    Exploring { scene: usize },
    TransitionOut { from: usize, to: usize, t: f32 },
    TransitionIn  { from: usize, to: usize, t: f32 },
}
```

- `TransitionOut`: la cámara sigue un spline Catmull-Rom desde su posición actual hasta el `exit_anchor`, con fade a negro creciente y resolución reducida.
- En `t == 1.0`: se hace el swap de escena activa.
- `TransitionIn`: cámara desde `entry_anchor` hasta la `default_camera` del destino, fade decreciente.

Las tres escenas se precargan en memoria al inicio (cada una con su BVH). Con geometría voxel el costo de memoria es bajo y se evita cualquier congelamiento al cambiar.

---

## 6. Plan de 4 semanas

| Semana | Entrega | Criterio de "listo" |
|---|---|---|
| **1** | Raytracer base: cubos (AABB), cámara orbital, material difuso, 1 luz, sombras duras, `rayon`, BVH | Una escena gris de ~500 cubos corriendo a **≥10 FPS** en release |
| **2** | Texturas por cara, materiales (reflexión, refracción, emisión), skybox, **diorama Odyssey completo** | Odyssey se ve terminado y presentable |
| **3** | **Diorama Galaxy + diorama NSMB Wii** + máquina de estados de escenas | Cambio de escena por tecla (sin transición aún) |
| **4** | Las 3 transiciones cinemáticas, audio, pulido, video de demo | Colchón de **2 días** para desastres |

### Regla de corte (decidida de antemano)

> Si el domingo de la Semana 3 no están las tres escenas listas, se entrega **2 dioramas + 2 transiciones** en ciclo A→B→A.

Por eso el `trait Scene` y la máquina de estados se escriben genéricos desde el día 1: recortar debe ser cambiar una lista, no reescribir el motor.

---

## 7. Orden de ataque (Semana 1, día a día)

1. `cargo init` + `Cargo.toml` con perfil release y dependencias
2. `Vec3`, `Ray`, `Framebuffer` con upscale a ventana
3. `Cube` (AABB) + intersección slab method → renderizar 1 cubo blanco
4. `Camera` orbital con mouse/teclado
5. Material difuso + 1 luz puntual + sombras duras (shadow ray)
6. `rayon` sobre el framebuffer → medir FPS
7. `Bvh` sobre los cubos → medir FPS otra vez
8. Escena gris de blockout de los 3 dioramas (solo cubos sin textura, para validar composición y silueta)

El paso 8 es clave: **validar la composición de los tres dioramas en gris antes de invertir tiempo en texturas.** Si un diorama no se lee bien en gris, tampoco se leerá con textura.

---

## 8. Checklist de entregables

- [ ] Tres dioramas (o dos, si aplica la regla de corte)
- [ ] Transiciones cinemáticas en ciclo cerrado
- [ ] Material difuso con textura
- [ ] Material reflectivo (metal)
- [ ] Material refractivo con Fresnel (agua / cristal)
- [ ] Material emisivo (estrellas, Launch Star)
- [ ] Skybox distinto por escena
- [ ] Sombras
- [ ] Cámara orbital controlable
- [ ] Rendimiento aceptable (≥10 FPS en la resolución de trabajo)
- [ ] Audio (música por escena + efecto de transición)
- [ ] Video de demostración
- [ ] README con controles y capturas

---

## 9. Nota legal

Los assets serán geometría voxel original, no modelos rippeados de los juegos. Las texturas deben ser propias o de licencia libre. Los nombres y personajes de Nintendo se usan en contexto académico y de homenaje; **no publicar el proyecto con fines comerciales**.

---

## 10. Decisión pendiente

¿Existe código de raytracer de un proyecto o laboratorio previo del curso?

- **Sí** → la Semana 1 arranca con ~60% hecho; el plazo pasa de ajustado a cómodo.
- **No** → se levanta el scaffold desde cero siguiendo el orden de ataque de la sección 7.
