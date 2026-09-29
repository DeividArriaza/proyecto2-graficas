# Proyecto 2 — Diorama con Raytracing

Entrega autocontenida de un diorama raytraced por CPU, construido únicamente con Rust estándar y cubos AABB. La escena ofrece tres mundos conectados: Mario Odyssey, Mario Galaxy y NSMB Wii.

## Requisitos cubiertos

- Tres dioramas seleccionables (`--scene 0`, `1` o `2`) y cambio cíclico con `n`, que genera una transición de fade-out/fade-in en seis frames PPM.
- Cubos con intersección slab, normales y UV por cara.
- Texturas procedurales propias para ladrillo, pasto, tubería, metal, agua, estrella y piedra; cada material tiene albedo, specular, transparencia y reflectividad.
- Reflexión (metal/agua), refracción con índice de refracción y transparencia (agua), emisión (estrella), iluminación difusa, specular y sombras.
- Skybox procedural distinto por escena.
- Cámara orbital con acercamiento/alejamiento y rotación; `r` rota el diorama.
- Render paralelo no requerido por la restricción de librerías externas; el programa usa una resolución configurable y mantiene el renderer determinista y portable.

## Ejecutar

```bash
cargo test
cargo run --release -- --scene 0 --width 320 --height 240 --output odyssey.ppm
cargo run --release -- --interactive --output interactivo.ppm
```

El resultado es un archivo PPM (`P3`), formato sin dependencias que se abre con GIMP, ImageMagick, Krita o un visor compatible. En modo interactivo, escribir comandos separados por espacios y ejecutar `render` para guardar el estado actual:

`a/d` orbitar · `w/s` elevar · `+/-` zoom · `r` girar diorama · `n` siguiente escena/transición · `q` salir. La transición crea `render.transition-00.ppm` a `render.transition-05.ppm`.

## Video de demostración

La consigna solicita enlazar un video en el README. Se generó localmente un GIF reproducible de la transición Odyssey → Galaxy en [artifacts/proyecto2-demo.gif](artifacts/proyecto2-demo.gif); no está publicado ni sustituye la presentación humana. No se inventa ningún enlace externo. Los renders y el reporte automatizado están en `artifacts/` (`final/`, `angles/`, `transitions/` y `render-verification.json`).

Para regenerar el GIF después de ejecutar una transición con `--interactive`, usando Pillow disponible en el entorno:

```bash
python3 -c 'from pathlib import Path; from PIL import Image; p=Path("artifacts/transitions"); f=[Image.open(x).convert("RGB") for x in sorted(p.glob("odyssey-to-galaxy.transition-*.ppm"))]; f[0].save("artifacts/proyecto2-demo.gif",save_all=True,append_images=f[1:],duration=180,loop=0,optimize=False)'
```

## Verificación visual manual

Abrir los tres `.ppm` generados y comprobar: diferenciación de skyboxes, texturas por cara, sombras, metal reflectivo, agua refractiva, estrella emisiva y control de cámara. La compilación y las pruebas automatizadas validan la geometría y los materiales, pero no sustituyen esta inspección visual ni la presentación en vivo.

Capturas finales: [Odyssey](artifacts/final/odyssey.png), [Galaxy](artifacts/final/galaxy.png) y [NSMB Wii](artifacts/final/nsmb-wii.png). La verificación automatizada de los 30 PPM está resumida en `artifacts/render-verification.json`: valida existencia, formato P3, dimensiones, conteo de píxeles, variación RGB, rango de canales y SHA-256.

## Fuentes y alcance

La implementación sigue `instrucciones.md` y `PLAN.md`. No usa crates ni librerías externas, respetando explícitamente la restricción de la rúbrica.
