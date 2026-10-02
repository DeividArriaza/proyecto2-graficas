# Audios propios

Los seis WAV de entrega están incluidos en el repositorio con estos nombres exactos:

| Mundo | Música/ambiente repetido | Efecto al salir del mundo |
| --- | --- | --- |
| Odyssey | `odyssey.wav` | `odyssey-transition.wav` |
| Galaxy | `galaxy.wav` | `galaxy-transition.wav` |
| Mario 64 | `mario64.wav` | `mario64-transition.wav` |

Desde la raíz del repositorio, `cargo run --release` los detecta automáticamente
cuando están los seis. Si la carpeta está incompleta, usa los sonidos sintetizados
y muestra un aviso. `B` silencia/reactiva. Un viaje directo usa el efecto del mundo
origen, no del destino. Las músicas se repiten mientras ese mundo está activo.

Formato recomendado: WAV PCM de 16 bits, mono o estéreo, 22050/44100/48000 Hz.
Los efectos deberían durar aproximadamente 0.8 segundos: se detienen al llegar.
No basta con renombrar un MP3 a `.wav`; exportalo realmente como WAV.
Usá archivos que tengas permiso de utilizar y normalizá el volumen de las pistas.

Para comprobar el sonido sin ventana:

```bash
cargo run --release -- --audio-demo
```

También podés guardar los archivos fuera del repositorio:

```bash
cargo run --release -- --audio-dir /ruta/a/mis/audios
```

La aplicación nunca modifica ni borra tus audios. Estos seis archivos aportados
para la evaluación se versionan mediante excepciones específicas en `.gitignore`;
los demás WAV y temporales siguen ignorados. Al clonar no hace falta descargarlos
por separado ni usar Git LFS. Sus tamaños, duraciones y hashes SHA-256 están en
`manifest.json`. El código de síntesis original se conserva como respaldo.
