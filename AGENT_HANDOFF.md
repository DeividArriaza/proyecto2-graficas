# Handoff para el próximo agente Codex

## Actualización vigente — 2026-10-01

Por solicitud posterior del usuario, el tercer mundo ahora es **Super Mario 64**, no
NSMB Wii. El ciclo vigente es Odyssey → Galaxy → Mario 64 → Odyssey. Las referencias
actuales `Mario_Galaxy2.png` y `Mario64.png` guían respectivamente el planetoide voxel
con rostro/gorra-jardín de Mario y el castillo de Peach con techos rojos, puente, agua
y jardines. Los modelos están en `src/worlds.rs`; los tres mundos activos usan sólo
cubos/AABB, manteniendo Launch Star y tubería como anclajes de viaje.

`src/audio.rs` sintetiza tres ambientes originales y tres efectos de 0.8 s; `B`
silencia/reactiva. No se incluyen las canciones de Nintendo: se cargan WAV propios
con `--audio-dir` (nombres en README). `--export-audio` exporta las pistas originales
sin sobrescribir archivos existentes; `--audio-demo` comprueba el ciclo sonoro.
La reproducción usa `pw-play` o `aplay` del sistema, sin nuevas dependencias Cargo;
si falla el dispositivo, el render continúa sin sonido. Headless y benchmarks no
inician audio. Los WAV temporales y procesos propios se limpian al cerrar normalmente.

`src/transitions.rs` prepara escenas/BVH una vez por viaje, actualiza los límites
de la BVH al despegar y reduce progresivamente la resolución hasta 160×120 durante
el movimiento, recuperando 320×240 en los extremos exactos. Conserva 0.8 s y todos
los anclajes. No volver a reconstruir escenas ni ordenar cubos por frame.
Los seis WAV propios en `assets/audio/` se detectan automáticamente al ejecutar
desde la raíz; los nombres y formato están en `assets/audio/README.md`.

README y los manifests actuales contienen los controles y resultados vigentes
(25 pruebas y benchmarks de órbita/transición superiores a 10 FPS de promedio).
Las menciones a NSMB Wii y los estados iniciales del resto de este handoff son
antecedentes históricos; no deben revertir esta actualización del usuario.

## Estado actual

El proyecto es un raytracer de CPU en Rust para tres dioramas: Mario Odyssey, Mario Galaxy y New Super Mario Bros. Wii. La geometría actual usa AABB/cubos, hay texturas procedurales por cara, sombras, reflexión, refracción, emisión y skyboxes por escena. La cámara permite órbita, elevación, zoom y rotación del diorama.

Commits relevantes en la línea local:

- `38ab6e6 feat: add explicit world transitions`: añadió la máquina de transiciones entre mundos, crossfade y cámara interpolada.
- `5fe1c05 feat: refine diorama visual readability`: revisión visual v3, cámaras/composición/iluminación y renders de revisión.
- También son antecedentes `e9ddd9a` (composición visual), `adf40e9` (renderer interactivo realtime) y `cfbf8f6` (entrega base).

Después del `fetch`, `origin/main` puede contener el commit `7e5c199 Add files via upload`, que incorpora los dos screenshots de referencia. No asumir que el estado local y el remoto son idénticos: comprobar siempre `status`, `log` y la divergencia antes de trabajar.

Los screenshots confirmados son `Screenshot From 2026-09-30 14-42-40.png` y `Screenshot From 2026-09-30 14-42-55.png`. La referencia muestra una Odyssey reconocible por su casco rojo/crema, copa alta, proa y plataformas laterales, barandas, mástil con bandera, faro frontal y elemento superior tipo globo/chimenea. El siguiente modelo debe perseguir esa lectura estructural, no una aproximación dominada por primitivas redondas.

## Requerimientos que mandan

- Implementar en Rust un raytracer por software/CPU. No sustituirlo por un motor gráfico que resuelva el render.
- Construir el diorama con cubos texturizados y AABB; resolver intersección slab y UV por la cara golpeada. La escena debe ser voxel y eficiente para raytracing.
- Implementar cinco materiales distintos como máximo puntuable, cada uno con textura propia y parámetros propios de albedo, specular, transparencia y reflectividad.
- Incluir refracción contextual en al menos un material y reflexión en al menos otro. Mantener también emisión donde ayude a estrellas/Launch Star o luces.
- Implementar un skybox por escena, con identidad visual claramente distinta.
- Permitir rotación del diorama y acercamiento/alejamiento de la cámara.
- Incluir el video de demostración en el README del repositorio.
- No añadir librerías externas al lenguaje fuera de lo permitido por el curso; preservar la implementación del raytracer y revisar cualquier dependencia antes de incorporarla.

## Prioridad absoluta: reconstrucción voxel de Odyssey

Antes de pulir materiales, reconstruir Odyssey como un **modelo voxel compuesto predominantemente de cuboides apilados, escalonados y solapados**. Usar los dos screenshots como guía de silueta, proporciones, jerarquía y distribución de masas. Debe leerse como una nave compacta vista en 3/4 incluso en blockout gris.

La composición mínima esperada es:

- casco principal crema y rojo, ancho y escalonado, con franjas/volúmenes horizontales;
- proa y plataformas laterales claramente extendidas;
- cubierta y base inferior en capas, con bordes y faldones formados por cuboides;
- cabina/copa roja alta, de volumen grande y reconocible, con bandas horizontales apiladas;
- ventanas blancas contrastantes, encastradas en el casco/cabina;
- faro frontal poligonal hecho de cubos, preferiblemente con aro, cuerpo y lente facetados;
- barandas y pasarelas visibles alrededor de la cubierta;
- mástil y bandera como elementos verticales delgados, sin convertirlos en el volumen dominante;
- cola/propulsores en la parte posterior, construidos con bloques y cilindros sólo como detalles secundarios si hacen falta;
- globo/chimenea superior, sostenido por una base voxel y subordinado a la silueta de la copa.

Evitar que esferas, cilindros o formas suaves sean la forma dominante. Los detalles redondos pueden existir como acentos, pero la masa, la cubierta, la copa, la proa y el faro deben provenir principalmente de cuboides AABB. La fidelidad buscada es la lectura visual de los screenshots mediante bloques, no una malla suave ni un modelo rippeado.

## Estándares visuales de Galaxy y NSMB

Galaxy debe diferenciarse por espacio estrellado, vacío profundo, planetoides y accidentes en órbita, iluminación/emisión de estrellas y Launch Star, y una composición inclinada que comunique escala cósmica. Los planetas pueden usar capas de cubos escalonados; no deben romper el estándar voxel ni parecer una colección de esferas lisas.

NSMB Wii debe tener cielo azul con nubes, colores saturados y lectura de plataforma: puerta/castillo, almenas, banderas, tuberías verdes, bloques de ladrillo, monedas y volúmenes claramente apilados. La iluminación debe ser más plana y alegre que Odyssey/Galaxy, con siluetas legibles y suficiente contraste entre fondo, suelo y objetos interactivos visuales.

En las tres escenas priorizar silueta, escala relativa, contraste de materiales, composición 3/4 y legibilidad a 320×240 antes que detalle ornamental.

## Ventana y framebuffer

- Render interno fijo de `320×240` como mínimo y escalarlo a una ventana amplia de al menos `960×720`; idealmente soportar `1280×960`.
- La ventana debe ser redimensionable y conservar una presentación nítida/proporcional del framebuffer interno.
- `F11` sólo debe declararse como fullscreen si la versión real de `minifb` usada lo soporta y se puede verificar. No inventar una API.
- Si `minifb` no expone una API fiable para minimizar, dejar la minimización al window manager del sistema y no simularla en la aplicación.
- La ruta headless debe seguir funcionando sin display y no debe generar temporales o `render.ppm` accidentalmente.

## Transiciones obligatorias

Las transiciones forman un ciclo cerrado Odyssey → Galaxy → NSMB Wii → Odyssey. `N` debe avanzar al siguiente mundo; `1`, `2` y `3` deben solicitar directamente Odyssey, Galaxy y NSMB Wii, respectivamente.

Cada transición dura exactamente `0.8 s`, usa `smoothstep`, crossfade de los dos renders y cámara interpolada/cinemática. No debe haber salto instantáneo de geometría, cámara o mundo. Durante una transición, las nuevas solicitudes deben tener una política determinista (ignorarlas o encolarlas, pero documentarla); al terminar, el destino debe quedar activo. Conservar los anclajes de salida/entrada y el fundido leve ya documentados en README.

## Criterios de aceptación y comandos

Antes de entregar, ejecutar y revisar:

```bash
cargo fmt --check
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
cargo run --release -- --benchmark --scene 0 --width 320 --height 240
```

El benchmark de órbita debe alcanzar al menos `10 FPS` en release a `320×240`; medir también la transición y conservar el resultado. Verificar headless con una ruta temporal explícita, por ejemplo `cargo run -- --headless --width 32 --height 24 --output "$tmpdir/smoke.ppm"`. Generar y revisar los manifests/renders de transición (`artifacts/transition-demo/manifest.json` o su equivalente v3), comprobando que los frames intermedios cambian y que el frame final declara el mundo destino.

La aceptación visual exige que Odyssey se reconozca por la silueta voxel solicitada, que Galaxy y NSMB mantengan sus identidades, que rotación/zoom funcionen, que los cinco materiales tengan textura y parámetros propios, y que refracción, reflexión, skyboxes y transiciones sean observables y verificables. Actualizar README con controles, video y resultados reproducibles.

## Disciplina Git

1. Hacer `git fetch --prune` antes de comenzar y revisar la divergencia con `origin/main`.
2. Preservar cambios existentes del usuario; no usar `reset --hard`, `checkout` destructivo ni sobrescribir trabajo ajeno.
3. No versionar secretos, `target/`, temporales, renders de prueba ni directorios de trabajo generados.
4. Revisar `git diff` y `git status`; stagear sólo los archivos intencionados.
5. Crear commits descriptivos, sin línea `Co-authored-by`.
6. Hacer push por SSH a `main`, sin force:

```bash
git push origin main
git ls-remote --heads origin main
git status --short --branch
git log -1 --oneline --decorate
```

Confirmar que el hash remoto coincide con el commit local y reportarlo al finalizar.

## Prompt sugerido para el agente

> Implementa primero la reconstrucción voxel de Mario Odyssey y la ventana grande, antes de iterar detalles secundarios. Usa los screenshots `Screenshot From 2026-09-30 14-42-40.png` y `Screenshot From 2026-09-30 14-42-55.png` como referencia: construye una nave predominantemente de cuboides AABB apilados/escalonados, con casco crema/rojo, proa, cubierta, cabina/copa roja alta con bandas, ventanas blancas, faro frontal poligonal de cubos, barandas, mástil/bandera, cola/propulsores y globo/chimenea superior. Evita que esferas o cilindros dominen. Mantén Rust y el raytracer CPU, render interno 320×240 escalado a una ventana redimensionable de al menos 960×720 (idealmente 1280×960), y conserva controles de órbita, zoom, rotación, materiales, reflexión, refracción, skyboxes y transiciones `N`/`1`/`2`/`3` de 0.8 s con smoothstep, crossfade y cámara interpolada. Primero valida la silueta de Odyssey en blockout voxel y la ventana; después pule texturas, materiales y detalles. Verifica con fmt, check, clippy, test, build release, headless, manifests y benchmark de al menos 10 FPS; preserva el trabajo existente y documenta todo en README.
