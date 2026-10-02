# Proyecto 2 — Dioramas con raytracing

Renderer de CPU en Rust para tres dioramas: Mario Odyssey, Mario Galaxy y Super Mario 64. Los tres modelos actuales se construyen exclusivamente con cubos/AABB texturizados. Conserva UV por cara, sombras, reflexión, refracción, emisión y skyboxes por escena.
Video de youtube: https://youtu.be/Jv6zhgX8zt4 

## Ejecutar

El comportamiento normal abre una ventana interactiva y la mantiene activa hasta cerrarla:

```bash
cargo run --release
```

El framebuffer se renderiza en paralelo por filas y se presenta después de cada frame. El render interno predeterminado es `320×240` y se escala a una ventana inicial de `1280×960`; la ventana es redimensionable y conserva la proporción con franjas negras mediante un escalador propio. Esto evita un fallo de `AspectRatioStretch` en el backend Wayland de minifb 0.26. No se crea ningún `render.ppm` al ejecutar así.

Controles:

| Tecla | Acción |
| --- | --- |
| Flechas o `WASD` mantenidas | Orbitar y elevar/bajar la cámara continuamente |
| `+` / `-` mantenidas | Zoom continuo |
| `R` mantenida | Rotar el diorama continuamente |
| `M` | Activar/desactivar inspección de materiales |
| `Tab` / `T` | Abrir la inspección directamente o recorrer los materiales presentes |
| `V` | Ocultar/mostrar todo el panel informativo con la misma tecla |
| `J` / `L` mantenidas | Cambiar el ángulo horizontal de la luz principal |
| `I` / `K` mantenidas | Subir/bajar el ángulo de la luz principal |
| `H` | Restablecer la iluminación original del mundo |
| `Q` / `E` | Quitar/añadir una energiluna en Odyssey (0–20) |
| `B` | Silenciar/reactivar música y efectos de sonido |
| `N` | Transición al siguiente mundo: Odyssey → Galaxy → Mario 64 → Odyssey |
| `1` / `2` / `3` | Transición directa a Odyssey / Galaxy / Mario 64 |
| `F11` | Sin acción: minifb 0.26 no ofrece una API pública y verificable para fullscreen |
| `Esc` o cerrar la ventana | Salir |

La minimización y cualquier fullscreen solicitado al sistema quedan a cargo del window manager. No se simula fullscreen con una API inexistente.

## Bloques y materiales

Odyssey conserva su nave voxel y ahora incorpora casas inspiradas en las referencias
`Desierto1.png` y `Desierto2.png`: fachadas turquesa/magenta, niveles amarillos,
ventanas enmarcadas, franjas decorativas y cúpulas hechas con cuboides escalonados.
El terreno rojizo se divide en una cuadrícula de bloques de 2 unidades con juntas;
las casas se construyen con bloques de 0.50/0.55 unidades. Cada tamaño responde a
una escala de construcción, y los bordes permanecen visibles al orbitar.

Todos los bloques usan intersección rayo-AABB y UV por cara. Las texturas se generan
en el proyecto mediante funciones de `u/v`; no son únicamente colores constantes.
La BVH descarta grupos de bloques que el rayo no cruza y acelera también las sombras.
Las figuras, matemáticas, texturas y efectos se implementan en Rust dentro del
proyecto; minifb presenta la ventana, Rayon paraleliza las filas y png exporta imágenes.
Los nuevos modelos están en `src/worlds.rs`; la síntesis y gestión de sonido, en
`src/audio.rs`. No se añadió ninguna dependencia de Cargo ni un motor de geometría.

Cinco materiales que pueden mostrarse durante la presentación:

| Nombre en el inspector (ID de exportación) | Textura propia | Albedo RGB | Especular | Transparencia | Reflectividad |
| --- | --- | --- | ---: | ---: | ---: |
| Arena (`sand`) | Granos y ondulaciones de arena rojiza | 0.89, 0.29, 0.12 | 0.04 | 0.00 | 0.00 |
| Estuco turquesa (`stucco-teal`) | Estuco granular turquesa de las casas | 0.02, 0.58, 0.51 | 0.10 | 0.00 | 0.00 |
| Ladrillo (`brick`) | Patrón rojo con juntas escalonadas | 0.82, 0.07, 0.03 | 0.18 | 0.00 | 0.04 |
| Metal (`metal`) | Paneles con variación de acabado | 0.78, 0.82, 0.90 | 0.70 | 0.00 | 0.34 |
| Agua (`water`) | Ondas procedurales | 0.08, 0.35, 0.52 | 0.85 | 0.62 | 0.16 |

El oasis usa agua refractiva con IOR 1.33, Fresnel de Schlick y reflexión interna total.
Su fondo tiene baldosas contrastantes para observar la transmisión/distorsión en el
render normal. El inspector es una vista diagnóstica: muestra la textura y la forma,
pero no calcula reflexión ni refracción. Para apreciar esos efectos, volver con `M`.

Otros materiales de Odyssey: `sandstone` (estratos), `cactus` (costillas),
`stucco-yellow` y `stucco-magenta` (patrones de estuco distintos), `cloud` (acabado
claro), `dark` (acabado oscuro) y `star` (dorado emisivo). Cada uno tiene su propia
configuración; la rúbrica puntúa como máximo cinco materiales, no limita la cantidad.

Al activar `M`, el material elegido conserva su textura con contornos dorados y el
resto de la escena se muestra gris. Antes `Tab` requería activar `M`: ahora `Tab` o
`T` abre directamente el inspector y las siguientes pulsaciones recorren los materiales.
El nombre, índice y los cuatro parámetros aparecen en un panel dentro de la ventana,
además del título y la terminal. `M` sale de la inspección. Rotación y zoom siguen disponibles; iniciar una transición sale de
la inspección y vuelve al render normal de los tres mundos.

Los 23 nombres de materiales se muestran en español en el panel, título y terminal
(por ejemplo, Césped, Tubería, Mampostería y Agua del lago). Los identificadores
de `--inspect-material` permanecen en inglés para conservar los comandos existentes.
`V` oculta todo el panel sin alterar cámara, luces, sonido, material seleccionado
ni modo de inspección; otra pulsación de `V` lo muestra de nuevo. La elección se
mantiene al cambiar de mundo y la tecla también funciona durante las transiciones.
Para volver al render normal, seguí usando `M`. Para grabar sin el panel desde el inicio:

```bash
cargo run --release -- --hide-hud
```

En headless, `--hide-hud` anula `--hud`, permitiendo capturas limpias.

La inspección también se puede exportar sin ventana:

```bash
cargo run --release -- --headless --scene 0 --inspect-material stucco-teal --width 640 --height 480 --output /tmp/inspeccion-casas.png
cargo run --release -- --headless --scene 0 --inspect-material water --width 640 --height 480 --output /tmp/inspeccion-agua.png
```

Se rechazan nombres desconocidos y materiales que no estén presentes en la escena.
Las capturas de inspección de entrega están en `artifacts/inspection/`.

## Luz y energilunas interactivas

`J/L` gira la luz principal alrededor del diorama y `I/K` cambia su elevación,
con límites para mantenerla sobre el horizonte. `H` restablece sus ángulos.
Las luces de relleno y contorno permanecen fijas. Los desplazamientos angulares se
conservan al viajar entre mundos. El panel muestra sus valores en grados.
Para apreciar sombras y reflejos hay que salir del inspector con `M`: este es diagnóstico,
no una vista de iluminación física.

En Odyssey, cada pulsación de `E` añade una energiluna y `Q` quita una, hasta
un máximo de 20. El globo sigue formado por 21 cubos y cambia de tamaño con
smoothstep durante **0.5 s**, manteniendo fija su base; no se convierte en esfera.
Comienza con 10 energilunas. El panel muestra cantidad y porcentaje de llenado.
El estado se conserva al regresar a Odyssey y se congela durante el viaje.
Es una interacción simbólica de llenado, no una mecánica de recolección de personajes.

Estos estados se pueden reproducir sin ventana; los ángulos son desplazamientos
en grados respecto a la luz original y `--hud` incluye el panel en el PNG:

```bash
cargo run --release -- --headless --scene 0 --moons 0 --hud --width 640 --height 480 --output /tmp/globo-vacio.png
cargo run --release -- --headless --scene 0 --moons 20 --hud --width 640 --height 480 --output /tmp/globo-lleno.png
cargo run --release -- --headless --scene 0 --light-azimuth 70 --light-elevation -10 --hud --width 640 --height 480 --output /tmp/luz-lateral.png
cargo run --release -- --headless --scene 0 --inspect-material water --hud --width 640 --height 480 --output /tmp/material-panel.png
```

Capturas y hashes: [controles interactivos](artifacts/controls/manifest.json).

## Galaxy y Mario 64 según las referencias

`Mario_Galaxy2.png` guía el nuevo planetoide con rostro de Mario: cabeza crema
facetada, orejas, nariz saliente, ojos, bigote verde, gorra-jardín con visera y banda
clara, insignia roja `M`, árboles y una casita en la copa. Cabeza, nariz y gorra son
envolventes voxel cerradas; no se usan esferas de render. La Launch Star emisiva y
las estrellas de la órbita también son cubos y conservan el viaje al siguiente mundo.

`Mario64.png` guía el reemplazo de NSMB Wii por el castillo de Peach: muros claros
en cursos de bloques, cuatro torres, torre central alta, techos rojos escalonados,
ventanas, entrada y medallón alusivo a Peach, puente, foso y lago azul, caminos,
jardines y árboles. La base queda delimitada como un diorama cuadrado. La tubería
verde sigue siendo el punto de salida hacia Odyssey; no hay personajes ni gameplay.

Materiales adicionales con textura y parámetros propios: `skin`, `castle`, `roof`,
`path`, `bark`, `lake-bed` y `lake-water`. El último mantiene refracción con IOR 1.33
y reflexión; su albedo azul y transparencia 0.32 distinguen el agua del lago del
agua del oasis. Se pueden recorrer con `Tab/T` y exportar con `--inspect-material`.

## Música y efectos por mundo

El modo interactivo reproduce por defecto los seis WAV aportados para la entrega,
incluidos en `assets/audio/` y disponibles al clonar el repositorio. Si faltan,
se sintetizan en Rust tres
ambientes originales diferentes de 12 segundos que se repiten: uno rítmico para
Odyssey, otro más etéreo para Galaxy y otro alegre para Mario 64. **No son las
canciones originales de Nintendo**: son únicamente el respaldo del proyecto.

Al iniciar un viaje se detiene la música origen y se dispara su efecto de 0.8 s:
ascenso de motor, brillo de Launch Star o descenso de tubería. Al llegar comienza
la música del destino. Las solicitudes ignoradas durante un viaje no disparan
efectos adicionales. `B` silencia tanto música como efectos; `--mute` inicia sin sonido.

La reproducción se delega a `pw-play` (PipeWire), con `aplay` (ALSA) como alternativa
si el primero no está instalado. Son herramientas del sistema, no implementan el
raytracer. Si no hay reproductor/dispositivo, el render continúa y el panel informa
que el audio no está disponible. Si el reproductor presente falla, se desactiva el
audio en esa ejecución; no se intenta cambiar de dispositivo automáticamente.
Los procesos son hijos propios, se recogen y se detienen al cerrar normalmente.
Los WAV sintetizados viven en un directorio temporal propio que se limpia al salir;
una terminación forzada del proceso puede impedir esa limpieza. Headless y benchmarks
no abren audio ni generan WAV.

Para usar tus canciones y efectos, colocá los seis archivos WAV en
[`assets/audio/`](assets/audio/README.md), dentro del repositorio. Al ejecutar
`cargo run --release` desde la raíz se detectan automáticamente cuando están los seis.
Si la carpeta está incompleta, se muestra un aviso y se conservan los sonidos originales.
También podés usar otra carpeta mediante `--audio-dir /ruta/a/mis/audios`.

| Ambiente | Efecto al salir |
| --- | --- |
| `odyssey.wav` | `odyssey-transition.wav` |
| `galaxy.wav` | `galaxy-transition.wav` |
| `mario64.wav` | `mario64-transition.wav` |

Formato recomendado: WAV PCM de 16 bits, mono o estéreo, a 22050/44100/48000 Hz.
Las músicas se repiten y los efectos deberían durar aproximadamente 0.8 s, pues
se detienen al llegar. El efecto corresponde al mundo del que salís.
No basta con renombrar un MP3: hay que convertirlo a WAV. Los seis WAV de entrega
sí se versionan; los demás WAV siguen ignorados. Sus hashes y duraciones están en
[`assets/audio/manifest.json`](assets/audio/manifest.json).

Podés exportar primero los seis audios originales y reemplazar únicamente las pistas
que quieras con grabaciones que tengas permiso de usar. El exportador rechaza destinos
con esos archivos existentes; la reproducción nunca escribe ni borra tu carpeta.

```bash
audio_export_dir=$(mktemp -d /tmp/mario-audio-XXXXXX)
cargo run --release -- --export-audio "$audio_export_dir"
cargo run --release -- --audio-dir "$audio_export_dir"
cargo run --release -- --audio-demo --audio-dir "$audio_export_dir"
cargo run --release -- --mute
```

`--audio-demo` prueba los tres ambientes y efectos sin abrir ventana; requiere una
salida de audio real. `--smoke-frames 30` cierra una ventana de prueba normalmente,
permitiendo recoger procesos y limpiar temporales. Los WAV de prueba no se versionan;
los parámetros y hashes PCM reproducibles están en `artifacts/audio/manifest.json`.

![Bloques de arena resaltados en el inspector](artifacts/inspection/sand.png)

[Estuco de las casas](artifacts/inspection/stucco-teal.png) ·
[Ladrillo](artifacts/inspection/brick.png) · [Metal](artifacts/inspection/metal.png) ·
[Agua](artifacts/inspection/water.png)

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

`--scene` acepta `0` (Odyssey), `1` (Galaxy) o `2` (Mario 64). El modo headless es útil para smoke tests y exportación; los renders de prueba deben mantenerse fuera del working tree o ignorados.

## Requisitos

- Rust estable y Cargo.
- En Linux, un servidor X11/Wayland para `cargo run`; en CI sin display puede usarse `xvfb-run`.
- `minifb` proporciona la ventana y el blit del framebuffer. El raytracer permanece implementado en el proyecto, sin motor gráfico externo.

## Arquitectura adoptada

Se consultó directamente la rama pública [`18-RT-06-REFLECTIONS`](https://github.com/menene/cc2018-2026-02-10/tree/18-RT-06-REFLECTIONS) del repositorio de referencia. Se adaptó su patrón técnico de `minifb`: `Vec<u32>` como framebuffer, `Window::is_open`, polling de teclado, cámara orbital y `update_with_buffer`. La adaptación conserva la arquitectura y los materiales propios del Proyecto 2; no se incorporó su historial ni se copió su escena.

La ventana rerenderiza cuando cambia la cámara o la escena; las teclas mantenidas usan
delta-time para movimiento continuo. `WorldTransition` conserva explícitamente mundo
origen/destino, progreso acotado `0..1`, cámara interpolada y duración fija de **0.8 s**.
Los frames intermedios renderizan ambos mundos, aplican easing smoothstep y componen sus framebuffers con
crossfade y un fundido a negro leve. La transición captura la cámara y la rotación actuales
al salir, de modo que orbitar, hacer zoom o girar antes de cambiar mundo no causa un
salto a la vista predeterminada. Durante la transición se suspende el control de cámara
y se usa el tiempo transcurrido real, sin el límite de delta-time del movimiento manual.
La salida depende del mundo origen: Odyssey despega físicamente con sus bloques y
un escape voxel, Galaxy acerca la cámara a una Launch Star construida con cubos y
un pulso emisivo, y Mario 64 acerca la cámara a una tubería con borde hueco. La llegada
parte de una cámara elevada y más distante y se asienta en la vista del destino.
Son motivos escénicos del viaje, sin personajes ni animación de Mario. La luz elegida
y el llenado del globo se conservan. Los extremos del crossfade coinciden exactamente
con los renders normales, incluso después de orbitar, girar o personalizar esos estados.
Mientras una transición está
activa, `N` y `1/2/3` se ignoran de forma determinista (no se encolan ni reinician); al
terminar, el mundo destino queda activo. Esc sigue cerrando la ventana y no se genera
`render.ppm` por defecto.
`src/transitions.rs` prepara las dos escenas/BVH una vez antes del viaje y reutiliza
sus buffers. El despegue actualiza límites de la BVH sin reconstruir ni ordenar
los bloques; el pulso de Galaxy parte de los valores originales para no acumularse.
La resolución se reduce progresivamente durante el primer 8 % del viaje hasta la
mitad de ancho y alto (**160×120** para el render habitual de 320×240), y vuelve
progresivamente a la resolución completa durante el último 8 %. Se escala el
resultado al framebuffer habitual; el HUD se dibuja después a resolución de ventana.
Se intercambia algo de detalle temporal por fluidez, sin alterar la calidad en reposo.
En los extremos sólo se renderiza el mundo visible y al llegar se reutiliza la escena
destino ya preparada. No se reproducen imágenes precalculadas ni se cambia la duración.
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
cargo run --release -- --benchmark --scene 1 --width 320 --height 240
cargo run --release -- --benchmark --scene 2 --width 320 --height 240
```

Medición del 2026-10-01 en release a 320×240 (render CPU, variable según el equipo):

| Mundo | Órbita FPS | Transición al siguiente FPS |
| --- | ---: | ---: |
| Odyssey | 72.73 | 97.86 |
| Galaxy | 116.50 | 82.75 |
| Mario 64 | 54.55 | 55.62 |

Antes de preparar las escenas y adaptar la resolución, la transición de Mario 64
a Odyssey promediaba 11.69 FPS; esta revisión midió 55.62 FPS. La ventana limita
la presentación a 60 FPS y los valores del benchmark no garantizan esa tasa en
otros equipos; la mejora se obtiene reduciendo trabajo, no alargando el viaje.

Cada benchmark mide ahora la transición que sale del mundo elegido, conservando su
cámara orbital final e incluyendo la preparación única de escenas/BVH, los renders
adaptativos y el escalado, no sólo el trazado.
Los FPS son promedios de la secuencia, no un mínimo por frame; varían según el equipo.
Las 27 pruebas incluyen la continuidad del primer y último
frame para las seis combinaciones de mundos, después de orbitar y rotar la escena.
La BVH se verifica comparando impactos y sombras contra la búsqueda lineal en los
tres mundos rotados. También se prueba que la inspección distingue materiales y
muestra sus parámetros; se prueban también el HUD, el llenado continuo y acotado del
globo, y la modificación de la luz principal sin alterar las luces auxiliares.
Las nuevas pruebas verifican los dos mundos voxel y sus anclajes de viaje, seis
audios distintos y acotados, duración de efectos, silencio, estado del ciclo y
limpieza de temporales propios sin sobrescribir archivos existentes.
También se comprueba la resolución adaptativa, la recuperación de calidad en los
extremos y la igualdad de la BVH actualizada respecto a una reconstruida, sin
acumulación de movimiento/emisión ni crecimiento de geometría durante el viaje.
Se verifican también las etiquetas españolas, los glifos con acentos y que ocultar
el panel deja intacta la imagen, sin una franja negra residual.

La pasada visual v3 en release a 320×240 midió 235.29 FPS (Odyssey), 285.71 FPS
(Galaxy) y 17.12 FPS (NSMB Wii), con 11 de 11 cambios de framebuffer durante la
órbita simulada. Odyssey→Galaxy durante la transición midió 107.46 FPS. El benchmark
mide render CPU y movimiento de cámara; la ventana requiere un display X11/Wayland.

La revisión desértica de Odyssey, con globo de energilunas más grande y base de
terreno, midió **31.25 FPS** en órbita y **23.85 FPS** en la transición a Galaxy a
320×240 en release, con 11/11 cambios de framebuffer. Estas mediciones corresponden
al render CPU; pueden variar según el equipo.

Las pruebas unitarias cubren intersección slab, existencia de geometría/materiales y órbita de cámara. El smoke test de exportación headless puede ejecutarse sin display con una resolución pequeña y una ruta temporal:

```bash
tmpdir=$(mktemp -d)
cargo run -- --headless --width 32 --height 24 --output "$tmpdir/smoke.ppm"
test -s "$tmpdir/smoke.ppm"
```

Para validar el ciclo sin display se puede generar una secuencia de quince
PNG (inicio, tres puntos intermedios y final para cada una de las tres transiciones) junto
con hashes, tiempos de render y FPS:

```bash
cargo run --release -- --transition-demo /tmp/proyecto2-transition-cycle --width 96 --height 72
```

La secuencia de entrega está en `artifacts/transition-cycle/manifest.json`; los hashes
intermedios son distintos dentro de cada transición y el último frame declara
`Mario Odyssey` como mundo final. Los frames extremos coinciden entre transiciones
consecutivas. La ruta del ejemplo genera otra revisión en `/tmp`.
Para medir rendimiento, `--benchmark` imprime tanto el FPS de órbita continua como
`TRANSITION_BENCHMARK` para el crossfade. En el benchmark release de 320×240 se exige
mantener al menos 10 FPS en órbita; la transición mide por separado sus dos renders por
frame. No se declara una ventana real validada cuando no hay display/Xvfb disponible.

Las capturas actuales están en `artifacts/final/`, con hashes y mediciones en su
`manifest.json`. El GIF anterior es histórico; el video de demostración está
enlazado al principio de este README. No se generan estos archivos automáticamente al abrir la ventana.

![Odyssey en el Reino de las Arenas](artifacts/final/odyssey.png)

[Captura Galaxy](artifacts/final/galaxy.png) · [Captura Mario 64](artifacts/final/mario64.png)

## Estado visual y límites

La revisión voxel posterior a v3 usa cámaras 3/4 específicas, iluminación key/fill/rim,
tone mapping y composiciones separadas: Odyssey está apoyada sobre un diorama cúbico
inspirado en el Reino de las Arenas, con arena rojiza dividida en bloques, estratos,
casas coloridas de cúpulas escalonadas, oasis, cactus, ruinas y cielo azul.
La nave conserva casco crema y
rojo escalonado, proa por capas, cabina/copa roja alta con bandas, ventanas blancas,
faro frontal facetado de cubos, barandas, mástil/bandera, cola con propulsores y un globo
superior formado por 21 cubos dorados apilados, con tamaño regulable mediante energilunas.
Odyssey no usa esferas: la silueta es un modelo
voxel AABB inspirado en las capturas de referencia. Galaxy muestra ahora el
planetoide de Mario y su gorra-jardín; Mario 64, el castillo de Peach y sus exteriores.
El antiguo render de NSMB queda como referencia histórica, no como escena activa.
Los PNG medidos, hashes y comparación con v2 están en
`artifacts/review-v3/`; la secuencia v3 está en `artifacts/transition-demo-v3/`.
Esos archivos son referencias históricas anteriores al globo ampliado y al entorno
desértico. Para obtener la escena actual, exportar un render con `--headless`.

El dip negro de transición está limitado a 7 %, para que el frame medio siga mostrando
los dos mundos. La calidad sigue siendo procedural y estilizada: no hay modelos
importados, texturas pintadas, antialiasing ni bloom. Las tres ventanas reales pasaron
una prueba de 30 ticks con audio y cierre normal (código 0), sin segfault.
El ciclo de ambientes y efectos pasó `--audio-demo` usando PipeWire. Wayland todavía
imprime avisos no fatales de decoración ausente y proxies al cerrar. Las teclas se validan
por sus funciones de estado y renders automatizados, no por una prueba manual completa.

La estética sigue siendo estilizada y procedural: no hay modelos ni texturas pintadas a
mano, bloom, antialiasing, ni assets de personajes. Es una mejora de legibilidad y
composición, no una reproducción exacta de los juegos.
