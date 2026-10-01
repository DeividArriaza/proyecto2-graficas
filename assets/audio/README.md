# Audios propios

Colocá aquí seis archivos WAV con estos nombres exactos, en minúsculas:

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

La aplicación nunca modifica ni borra tus audios. Los WAV están ignorados por Git
para evitar subir grabaciones grandes o sin permiso; copiarlos aquí no los publica.
No se incluyen canciones originales de Nintendo en el proyecto.
