<div align="center">

<img src="docs/icon.png" width="128" alt="Ícono de Voltkit: un rayo que cruza una llave inglesa">

# Voltkit

[English](README.md) · **Español** · [Deutsch](README.de.md) · [Français](README.fr.md) · [简体中文](README.zh-CN.md)

**Herramientas potentes para tu escritorio Linux.**
OCR de pantalla, un lector de códigos QR, un selector de color, una regla de pantalla, un compresor de imágenes y
videos, un expansor de texto, Pegado inteligente, Mantener activo, un editor de hosts y entorno, un buscador de
“¿qué lo está usando?”, un administrador de permisos de apps, un diagnóstico de suspensión, un informe del sistema,
un detective de atajos y un ajuste de GPU, en una sola app.

[![Última versión](https://img.shields.io/github/v/release/arios4k/voltkit?label=versi%C3%B3n&color=7c3aed)](https://github.com/arios4k/voltkit/releases/latest)
[![CI](https://github.com/arios4k/voltkit/actions/workflows/ci.yml/badge.svg)](https://github.com/arios4k/voltkit/actions/workflows/ci.yml)
[![Licencia: MIT](https://img.shields.io/badge/licencia-MIT-blue)](LICENSE)
[![Instalar con apt](https://img.shields.io/badge/instalar-apt-orange)](#instalación-ubuntu-debian-y-derivadas)
[![Instalar en Fedora](https://img.shields.io/badge/instalar-Fedora-51a2da)](#instalación-fedora)
[![Hecho con Rust](https://img.shields.io/badge/hecho%20con-Rust-dea584?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Apoyar en PayPal](https://img.shields.io/badge/PayPal-Support-00457C?logo=paypal&logoColor=white)](https://paypal.me/AlexisRios)

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/color-dark.png?v=0.11.0">
  <img src="docs/screenshots/color-light.png?v=0.11.0" alt="El selector de color de Voltkit mostrando #7C3AED con sus valores HEX, RGB y HSL y las calificaciones de contraste WCAG">
</picture>

</div>

> Voltkit está disponible en español, inglés, alemán, francés y chino simplificado, y usa el idioma de tu sistema automáticamente: la app,
> la línea de comandos y las páginas del manual (`man voltkit`). Las capturas de esta página son de la versión
> en inglés.

## Una app en lugar de dieciocho

Tener estas herramientas básicas en Linux suele significar buscar un applet distinto para cada tarea, cada uno con
su propio aspecto, ajustes y atajos. Voltkit las reúne en una app GTK4 nativa con un solo conjunto de atajos:

| En lugar de… | Voltkit te da | Atajo |
|---|---|---|
| nada: no había una pantalla para esto | **Permisos de apps**: cambia u olvida las respuestas que diste a las apps que pidieron capturas de pantalla, la cámara, tu ubicación y más | — |
| un compresor de imágenes aparte, más conjuros de ffmpeg | **Compresor de archivos**: suelta archivos PNG, JPEG, WebP o MP4 para quitarles los metadatos y reducir su tamaño | arrastrar y soltar |
| un selector de color aparte | **Selector de color**: un cuentagotas para cualquier píxel, HEX/RGB/HSL, revisión de contraste WCAG e historial | <kbd>Super</kbd>+<kbd>Mayús</kbd>+<kbd>C</kbd> |
| bucles de shell con `mv` o un renombrador aparte | **Renombrar en lote**: renombra muchos archivos a la vez con buscar y reemplazar, expresiones regulares y numeración, con vista previa y deshacer | — |
| un redimensionador de imágenes aparte | **Redimensionar imágenes**: cambia el tamaño de muchas imágenes a la vez, conservando su forma y sin tocar los originales | — |
| una herramienta aparte para ajustar la GPU, más utilidades de cada fabricante | **Ajuste de GPU**: monitorea tarjetas gráficas NVIDIA, AMD e Intel y cambia límites de potencia, ventiladores y curvas de ventilador, temperaturas objetivo, frecuencias y voltaje, con cada cambio en vista previa y deshecho si no lo confirmas | — |
| editar `/etc/hosts` y `~/.profile` en una terminal | **Hosts y entorno**: activa y desactiva entradas de hosts y administra las variables de entorno de tu sesión | — |
| una extensión del shell para mantener la pantalla encendida | **Mantener activo**: evita que tu computadora se suspenda o se bloquee de 30 minutos a 4 horas, o hasta que lo apagues | — |
| un lápiz de presentación aparte | **Zoom y anotaciones**: congela la pantalla, acércala y dibuja sobre ella con lápiz, flechas, recuadros y texto | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>Z</kbd> |
| un lector QR en tu teléfono o una página web de QR | **Código QR**: lee un código QR en cualquier parte de la pantalla, o convierte texto, enlaces y datos de Wi-Fi en uno | <kbd>Super</kbd>+<kbd>Mayús</kbd>+<kbd>Q</kbd> |
| una regla de pantalla aparte, o capturas y un editor de imágenes | **Regla de pantalla**: congela la pantalla y mide lo que sea en píxeles | <kbd>Super</kbd>+<kbd>Mayús</kbd>+<kbd>R</kbd> |
| adivinar por qué un atajo de teclado no hace nada | **Detective de atajos**: presiona cualquier atajo para ver a quién pertenece, encontrar conflictos, elegir una combinación libre y cambiarlo | — |
| convertidores en línea para JSON, tablas y mayúsculas | **Pegado inteligente**: pega el portapapeles convertido: JSON ordenado, una tabla de hoja de cálculo como Markdown, un enlace sin rastreo, texto de un PDF sin líneas cortadas, o el texto de una imagen copiada | <kbd>Super</kbd>+<kbd>Mayús</kbd>+<kbd>V</kbd> |
| un montón de comandos de terminal para responder “¿qué hardware tengo?” | **Informe del sistema**: tu hardware explicado, con revisiones y soluciones de drivers y de salud de las unidades, temperaturas, actualizaciones de firmware, ampliaciones de memoria, qué cambió desde una actualización y un informe seguro para compartir | — |
| un expansor de texto aparte | **Expansión de texto**: un selector de fragmentos con búsqueda, y disparadores opcionales al escribir como `;date` | <kbd>Super</kbd>+<kbd>Mayús</kbd>+<kbd>E</kbd> |
| una app de OCR aparte | **Extractor de texto**: arrastra un recuadro alrededor de cualquier texto en pantalla, incluso en videos e imágenes, y llega a tu portapapeles | <kbd>Super</kbd>+<kbd>Mayús</kbd>+<kbd>T</kbd> |
| `lsof`, `fuser` y `ss` en una terminal | **¿Qué lo está usando?**: mira qué apps mantienen ocupado un archivo, una carpeta, una unidad o un puerto, y ciérralas | — |
| buscar en los registros del sistema en una terminal | **¿Por qué no se suspendió?**: mira qué mantiene activa la computadora, qué la despertó y cuánta batería usó mientras estaba suspendida | — |

## Funciones

### 🛡️ Permisos de apps

Cuando una app pide tomar capturas de pantalla, usar la cámara o el micrófono, o ver tu ubicación, el escritorio
recuerda tu respuesta. Permisos de apps las muestra por app: mueve
un interruptor para cambiar una, u olvídala para que la app vuelva a preguntar.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/permissions-dark.png?v=0.11.0">
  <img src="docs/screenshots/permissions-light.png?v=0.11.0" alt="La página Permisos de apps con las apps y sus permisos de cámara y capturas de pantalla">
</picture>

### 📦 Compresor de archivos

Suelta archivos en la ventana o ejecuta `voltkit compress`. Voltkit quita los datos de ubicación y otros metadatos,
comprime con la mejor herramienta para cada formato y escribe `nombre.min.ext` junto al original, que nunca
toca. En nuestras pruebas: JPEG −43 %, PNG −23 % (sin pérdida), WebP −60 %, MP4 −77 %.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/compress-dark.png?v=0.11.0">
  <img src="docs/screenshots/compress-light.png?v=0.11.0" alt="El compresor de archivos con cuatro archivos comprimidos y el espacio ahorrado">
</picture>

### ✏️ Renombrar en lote

Suelta archivos o carpetas, di qué cambiar y mira la **vista previa** antes de que pase nada: buscar y reemplazar, expresiones regulares con grupos `$1`, numeración con `{n:3}`, mayúsculas y minúsculas, y el nombre o solo la extensión. Los conflictos (dos archivos con el mismo nombre, un nombre ocupado) se señalan primero, un lote se renombra **todo o nada** y el último lote se puede **deshacer**. Funciona igual en la terminal: `voltkit rename *.jpg --find IMG --replace Vacaciones --apply`.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/rename-dark.png?v=0.11.0">
  <img src="docs/screenshots/rename-light.png?v=0.11.0" alt="Renombrar en lote con una vista previa de cinco fotos que se renombran">
</picture>

### 🖼️ Redimensionar imágenes

Añade imágenes, elige un tamaño (un lado más largo, un porcentaje, o un ancho y alto que se ajusta, rellena o estira, o pulsa una idea como «Correo o chat») y pulsa **Redimensionar**: las copias aparecen junto a los originales, que nunca se tocan. Las fotos salen derechas, la calidad de WebP y JPEG la eliges tú y el tamaño nunca crece salvo que lo permitas. `voltkit resize fotos/*.jpg --longest-side 1280`.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/resize-dark.png?v=0.11.0">
  <img src="docs/screenshots/resize-light.png?v=0.11.0" alt="Redimensionar imágenes con tres imágenes ajustadas a 1920 píxeles">
</picture>

### 🎨 Selector de color

Toma cualquier píxel con <kbd>Super</kbd>+<kbd>Mayús</kbd>+<kbd>C</kbd> y el código HEX ya está en tu
portapapeles. La página de color convierte entre HEX, RGB y HSL, califica el contraste contra blanco y negro
según las normas WCAG (AA / AAA) y recuerda tus colores recientes.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/color-dark.png?v=0.11.0">
  <img src="docs/screenshots/color-light.png?v=0.11.0" alt="El selector de color con vista previa, formatos HEX/RGB/HSL, calificaciones de contraste y colores guardados">
</picture>

### 🎛️ Ajuste de GPU

Mira en vivo tus tarjetas gráficas NVIDIA, AMD e Intel, con historial, y cambia lo que el hardware permite: el límite
de potencia, una velocidad fija del ventilador o una curva, la temperatura que busca la tarjeta, desplazamientos o
límites de frecuencia y el voltaje. **Qué admite esta tarjeta** lista cada control tal como lo informa el controlador y
explica por qué falta alguno y qué podría activarlo (una GPU de portátil que oculta su ventilador, o una tarjeta AMD que
necesita `amdgpu.ppfeaturemask`). Los cambios se comparan con los límites de la propia tarjeta, se muestran en vista
previa, se aplican con un ayudante con permisos de administrador y **se deshacen si no los confirmas**: cerrar la app o
un fallo también los revierte. Los ajustes guardados los aplica al iniciar un servicio en segundo plano, que omite un
perfil que nunca terminó de iniciar, para que uno defectuoso no te deje atrapado en un ciclo. `voltkit gpu` muestra el
mismo informe en una terminal.

Edita las curvas de ventilador arrastrando puntos en un gráfico y elige qué temperatura siguen y qué ventiladores controlan. Guarda ajustes como **perfiles** que cambian solos: con batería, mientras se ejecuta una app, bajo una carga sostenida o cuando la tarjeta se calienta. **Graba una sesión** mientras corre un juego o una prueba de rendimiento y compara dos perfiles lado a lado: temperatura, potencia, energía y eficiencia (frecuencia activa por vatio, o fotogramas por vatio si se registraron). `voltkit gpu check` compara las lecturas de Voltkit con `nvidia-smi` y mide cuánto cuesta monitorear.

¿Lo probaste en tu tarjeta? Un [reporte de hardware](https://github.com/arios4k/voltkit/issues/new?template=gpu-hardware-report.yml) con lo que funcionó y lo que no es la ayuda más útil por ahora.

Aplicar cambios es nuevo y hasta ahora solo se ha probado con hardware simulado; consulta
[docs/GPU_TUNER.md](docs/GPU_TUNER.md) para saber qué funciona, cómo se mantiene seguro y qué falta por construir.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/gpu-dark.png?v=0.11.0">
  <img src="docs/screenshots/gpu-light.png?v=0.11.0" alt="Ajuste de GPU, pestaña Monitorización: lecturas en vivo con historial, por qué se frena el reloj y qué admite la tarjeta (datos simulados)">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/gpu-2-dark.png?v=0.11.0">
  <img src="docs/screenshots/gpu-2-light.png?v=0.11.0" alt="Ajuste de GPU, pestaña Ajustar: límite de potencia, editor de curva del ventilador y demás controles (datos simulados)">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/gpu-3-dark.png?v=0.11.0">
  <img src="docs/screenshots/gpu-3-light.png?v=0.11.0" alt="Ajuste de GPU, pestaña Perfiles: perfiles guardados, reglas automáticas, compartir e importar (datos simulados)">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/gpu-4-dark.png?v=0.11.0">
  <img src="docs/screenshots/gpu-4-light.png?v=0.11.0" alt="Ajuste de GPU, pestaña Sesiones: sesiones grabadas, precio de la electricidad, sugerencia de límite de potencia y comparación (datos simulados)">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/gpu-vf-dark.png?v=0.11.0">
  <img src="docs/screenshots/gpu-vf-light.png?v=0.11.0" alt="Editor de curva de voltaje y frecuencia del ajuste de GPU con puntos arrastrables (datos simulados)">
</picture>

<sub>La captura usa datos simulados de tarjetas inventadas.</sub>

### 📝 Hosts y entorno

Activa y desactiva entradas de `/etc/hosts`, agrega `myapp.test → 127.0.0.1` para desarrollo local y define
variables de entorno como `EDITOR` o una carpeta extra en `PATH` para toda tu sesión de escritorio. Las entradas
del sistema están protegidas y cada vez que se guarda hosts queda una copia de seguridad.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/system-dark.png?v=0.11.0">
  <img src="docs/screenshots/system-light.png?v=0.11.0" alt="La página Hosts y entorno con entradas de hosts e interruptores">
</picture>

### ☕ Mantener activo

¿Estás viendo, presentando o descargando algo? Enciende Mantener activo por 30 minutos, unas horas o hasta que lo
apagues, y tu computadora no se suspenderá ni se bloqueará. Sigue funcionando después de cerrar Voltkit y se
apaga sola cuando se acaba el tiempo.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/awake-dark.png?v=0.11.0">
  <img src="docs/screenshots/awake-light.png?v=0.11.0" alt="Mantener activo con su estado actual y los controles de duración">
</picture>

### 📷 Código QR

Presiona <kbd>Super</kbd>+<kbd>Mayús</kbd>+<kbd>Q</kbd> y arrastra un recuadro alrededor de un código QR en un
video, una diapositiva o una página web: su texto o enlace queda en tu portapapeles. En sentido contrario,
escribe un enlace, un texto o los datos de tu Wi-Fi y escanea el código con tu teléfono. Cópialo como imagen o
guárdalo como PNG para imprimir. Si quieres, agrega tu propio logo pequeño en el centro: Voltkit vuelve a leer
el código para comprobar que todavía se puede escanear.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/qr-dark.png?v=0.11.0">
  <img src="docs/screenshots/qr-light.png?v=0.11.0" alt="La página Código QR con un código generado para un enlace">
</picture>

### 📏 Regla de pantalla

<kbd>Super</kbd>+<kbd>Mayús</kbd>+<kbd>R</kbd> congela la pantalla. Arrastra para medir anchos y altos en píxeles
reales (mantén Mayús para una línea recta) y presiona Ctrl+C para copiar el tamaño.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/ruler-dark.png?v=0.11.0">
  <img src="docs/screenshots/ruler-light.png?v=0.11.0" alt="La regla de pantalla con su botón para abrirla y los controles de medición">
</picture>

### 🔍 Zoom y anotaciones

<kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>Z</kbd> congela la pantalla. Desplaza para acercar, arrastra para dibujar, mantén Mayús para una línea, Ctrl para un rectángulo, Tab para una elipse, Ctrl+Mayús para una flecha, pulsa T para escribir, R/G/B/O/Y/P para el color, W/K para una pizarra blanca o negra, y Ctrl+C o Ctrl+S para copiar o guardar la imagen. Las imágenes guardadas van a `Imágenes/Voltkit` o a la carpeta que elijas en la página de la herramienta (Ctrl+Mayús+S guarda una en otro sitio).

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/zoom-overlay-dark.png?v=0.11.0">
  <img src="docs/screenshots/zoom-overlay-light.png?v=0.11.0" alt="Zoom y anotaciones: una flecha, un círculo, texto y un marcador dibujados sobre una pantalla congelada">
</picture>

### ⌨️ Detective de atajos

¿Presionaste un atajo y no pasó nada, o pasó otra cosa? Presiónalo en **Detective de atajos** para ver cada
acción que lo usa: el gestor de ventanas, GNOME Shell, las teclas del sistema, tus atajos personalizados, las
extensiones del shell y el propio Voltkit. Los conflictos aparecen listados, cada uno con combinaciones libres a
las que cambiar, y cualquier atajo se puede cambiar, desactivar o restablecer ahí mismo. Funciona en GNOME y en
los escritorios construidos como él (Cinnamon, Budgie, MATE).

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/shortcuts-dark.png?v=0.11.0">
  <img src="docs/screenshots/shortcuts-light.png?v=0.11.0" alt="Detective de atajos mostrando que dos acciones usan Super+V">
</picture>

### 📋 Pegado inteligente

Presiona <kbd>Super</kbd>+<kbd>Mayús</kbd>+<kbd>V</kbd> en lugar de <kbd>Ctrl</kbd>+<kbd>V</kbd> y elige cómo pegar
lo que copiaste: JSON ordenado o compactado, un rango de una hoja de cálculo como tabla Markdown o CSV, un enlace
sin sus parámetros de rastreo, un párrafo de un PDF sin sus líneas cortadas, una lista con viñetas o números, o
texto en MAYÚSCULAS, Tipo Título, snake_case y más. ¿Copiaste una captura de pantalla? Pega el texto que contiene.
Solo se ofrecen las conversiones que sirven para lo que copiaste, cada una con vista previa, y tu portapapeles
queda como estaba. Las mismas conversiones funcionan en scripts: `wl-paste | voltkit convert markdown-table`.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/smartpaste-dark.png?v=0.11.0">
  <img src="docs/screenshots/smartpaste-light.png?v=0.11.0" alt="La página Pegado inteligente convirtiendo un rango de una hoja de cálculo en una tabla Markdown y en CSV">
</picture>

### 🖥️ Informe del sistema

No es solo una lista de especificaciones: **Informe del sistema** revisa cada pieza de hardware y te dice qué
significa. ¿La tarjeta gráfica tiene el driver correcto (y si no, el comando exacto para arreglarlo)? ¿La unidad
está sana, y qué tan desgastada? ¿El procesador está muy caliente o se está ralentizando? ¿Hay una actualización
de BIOS o de firmware esperando? ¿Cuántas ranuras de memoria están libres, y cuánta memoria admite esta computadora?

Voltkit también guarda una instantánea diaria del hardware, los drivers, el kernel y el firmware, así que cuando
algo se rompe después de una actualización, **Qué cambió** muestra exactamente qué es distinto. Cuando necesites
ayuda, **Copiar informe** te da un resumen ordenado para un foro o un reporte de errores, sin números de serie,
nombres ni direcciones de red.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/sysreport-dark.png?v=0.11.0">
  <img src="docs/screenshots/sysreport-light.png?v=0.11.0" alt="La página Informe del sistema con revisiones, cambios recientes y el hardware de la computadora">
</picture>

### ⌨️ Expansión de texto

Presiona <kbd>Super</kbd>+<kbd>Mayús</kbd>+<kbd>E</kbd>, busca, presiona Enter y el fragmento se pega en la app
que estabas usando. Los fragmentos pueden incluir la fecha en cualquier formato, tu portapapeles y campos para
completar. ¿Prefieres escribir? Enciende los disparadores al escribir y `;sig` se convierte en tu firma en
cualquier app. Agrega, edita y elimina fragmentos en la misma app: los botones Insertar agregan la fecha, la hora,
el portapapeles o un campo para completar, y una vista previa en vivo muestra exactamente lo que se pegará.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/snippets-dark.png?v=0.11.0">
  <img src="docs/screenshots/snippets-light.png?v=0.11.0" alt="Expansión de texto con fragmentos con búsqueda y controles para insertar">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/picker-dark.png?v=0.11.0">
  <img src="docs/screenshots/picker-light.png?v=0.11.0" alt="El selector de fragmentos con fragmentos y sus vistas previas">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/snippet-editor-dark.png?v=0.11.0">
  <img src="docs/screenshots/snippet-editor-light.png?v=0.11.0" alt="El editor de fragmentos con un disparador, el texto con botones para insertar fecha, hora, portapapeles y campos para completar, y una vista previa en vivo">
</picture>

### 🔍 Extractor de texto

La contraseña del Wi-Fi en una diapositiva de una conferencia, un comando en un video tutorial, un error en una
captura de pantalla: presiona <kbd>Super</kbd>+<kbd>Mayús</kbd>+<kbd>T</kbd>, arrastra un recuadro y pega. Usa
Tesseract, compatible con [más de 100 idiomas](https://tesseract-ocr.github.io/tessdoc/Data-Files-in-different-versions.html)
(para español instala `tesseract-ocr-spa` y usa `spa` como idioma).

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/ocr-dark.png?v=0.11.0">
  <img src="docs/screenshots/ocr-light.png?v=0.11.0" alt="El extractor de texto mostrando el texto leído de una foto de una diapositiva: una contraseña de Wi-Fi y un enlace">
</picture>

### 🔎 ¿Qué lo está usando?

“Target is busy” al expulsar una memoria USB, “address already in use” al iniciar un servidor de desarrollo:
elige la unidad, el archivo, la carpeta o el puerto y mira qué apps lo ocupan y qué tienen abierto. Cierra una
app con un clic (primero se le pide amablemente que se cierre, y solo se fuerza si no lo hace), o deja que
**Expulsar** las cierre y expulse la unidad por ti. Las partes del propio sistema nunca se ofrecen para cerrar.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/using-dark.png?v=0.11.0">
  <img src="docs/screenshots/using-light.png?v=0.11.0" alt="La página ¿Qué lo está usando? con las apps que mantienen ocupada una unidad USB">
</picture>

### 🌙 ¿Por qué no se suspendió?

¿La laptop calentita en tu mochila, o la batería vacía en la mañana? Mira qué mantiene activa la computadora
ahora mismo (una app, un video, Mantener activo, un ajuste de energía, un monitor externo), cada suspensión de la
última semana con qué la inició y la terminó y cuánta batería usó, y los dispositivos USB que pueden despertarla,
con un interruptor para frenar el que la sigue despertando.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/sleep-dark.png?v=0.11.0">
  <img src="docs/screenshots/sleep-light.png?v=0.11.0" alt="La página ¿Por qué no se suspendió? con lo que mantiene activa la computadora y las suspensiones recientes">
</picture>

## Por qué Voltkit

- **Hecho para Wayland.** Usa los portales del propio escritorio para capturas, el cuentagotas y el pegado, así
  que no hay trucos de X11 ni extensiones del shell.
- **Privado por diseño.** Todo funciona localmente, sin acceso a la red y sin telemetría. Los disparadores al
  escribir solo guardan en memoria los últimos caracteres y nunca registran ni guardan lo que escribes.
- **Pequeño y nativo.** Un solo binario de 4 MB (un paquete de 1.4 MB), GTK4 + libadwaita, modo claro y oscuro.
- **Programable.** Cada función también es un comando (`voltkit ocr`, `voltkit color '#3584e4'`,
  `voltkit compress *.jpg`), con páginas del manual para todos.
- **Se revisa solo.** `voltkit doctor` te dice exactamente qué falta y el comando para instalarlo.
- **Paquetes firmados.** Instala y actualiza con apt desde un repositorio firmado con GPG.
- **En tu idioma.** Español, inglés, alemán, francés y chino simplificado, según el idioma de tu sistema.

## Preguntas frecuentes

**¿Funciona en KDE, Sway o Hyprland?**
Voltkit se desarrolla y prueba en GNOME (Zorin OS 18). Usa portales XDG estándar, así que KDE Plasma y otros
escritorios deberían funcionar, pero todavía no se han probado. ¡Los reportes son bienvenidos! En escritorios sin
el portal GlobalShortcuts, asigna tú mismo `voltkit trigger ocr|color|snippets` a unas teclas.

**¿Es seguro encender los disparadores al escribir?**
Vienen apagados. Wayland oculta las pulsaciones de teclas entre apps, así que los disparadores al escribir
necesitan acceso al teclado, que permites una vez con tu contraseña al encenderlos. Voltkit solo guarda en memoria
los últimos caracteres y nunca guarda ni envía lo que escribes. El selector de fragmentos funciona sin nada de eso.

**¿Cómo cambio el idioma?**
Voltkit sigue el idioma de tu sesión. Para probar otro solo para Voltkit, ábrelo desde una terminal con
`LANGUAGE=en voltkit` (o `es`, `de`, `fr`, `zh_CN`).

<sub>Capturas tomadas en Zorin OS 18 con las paletas clara y oscura de Voltkit. Se regeneran con
`docs/screenshots/capture.sh` (o `docs/screenshots/capture.sh es` para la app en español; también `de`, `fr` y `zh_CN`).</sub>

## Instalación (Ubuntu, Debian y derivadas)

Para Ubuntu 24.04+, Debian 13+, Linux Mint 22, Zorin OS 18, Pop!_OS 24.04 y otras derivadas en Intel/AMD de 64 bits:

```bash
sudo curl -fsSLo /usr/share/keyrings/voltkit-archive-keyring.gpg \
  https://arios4k.github.io/voltkit/voltkit-archive-keyring.gpg
echo "deb [signed-by=/usr/share/keyrings/voltkit-archive-keyring.gpg] https://arios4k.github.io/voltkit stable main" \
  | sudo tee /etc/apt/sources.list.d/voltkit.list
sudo apt update && sudo apt install voltkit
```

apt instala las herramientas necesarias (Tesseract, wl-clipboard, el portal del escritorio) junto con Voltkit, y
las actualizaciones llegan con `sudo apt upgrade`.

Luego abre Voltkit desde tus apps. Eso es todo: inicia su servicio en segundo plano (que también se inicia en cada
inicio de sesión de ahí en adelante) y, en GNOME, agrega sus atajos de teclado. `voltkit doctor` revisa que todo
esté en orden. Escribir disparadores como `;date` en cualquier lado es opcional: enciende **Disparadores al
escribir** en la página Expansión de texto (pide tu contraseña una vez y funciona de inmediato).

## Instalación (Fedora)

Para Fedora 43 y posteriores en Intel/AMD de 64 bits, desde el [repositorio COPR](https://copr.fedorainfracloud.org/coprs/arios/voltkit/):

```bash
sudo dnf copr enable arios/voltkit
sudo dnf install voltkit
```

Luego abre Voltkit desde tus apps; se configura solo de la misma forma. Las actualizaciones llegan con
`sudo dnf upgrade`.

## Instalación (desde el código fuente)

Requisitos: Rust 1.85+, GTK ≥ 4.14, libadwaita ≥ 1.5.

```bash
# Debian/Ubuntu
sudo apt install libgtk-4-dev libadwaita-1-dev
# Fedora
sudo dnf install gtk4-devel libadwaita-devel
# Arch
sudo pacman -S gtk4 libadwaita
```

```bash
cargo install --path .     # se instala en ~/.cargo/bin/voltkit
voltkit
```

Instala el binario antes de configurar los atajos o el inicio automático: ambos guardan la ruta del binario.

Para agregar Voltkit a tu cuadrícula de apps con su ícono:

```bash
install -Dm644 data/icons/io.github.arios4k.Voltkit.svg ~/.local/share/icons/hicolor/scalable/apps/io.github.arios4k.Voltkit.svg
install -Dm644 data/icons/io.github.arios4k.Voltkit-symbolic.svg ~/.local/share/icons/hicolor/symbolic/apps/io.github.arios4k.Voltkit-symbolic.svg
sed "s|^Exec=voltkit|Exec=$HOME/.cargo/bin/voltkit|" data/io.github.arios4k.Voltkit.desktop > ~/.local/share/applications/io.github.arios4k.Voltkit.desktop
```

## Servicio en segundo plano y atajos globales

```bash
voltkit daemon                    # se ejecuta en primer plano
voltkit daemon enable-autostart   # se inicia en cada inicio de sesión
voltkit daemon status             # ¿se está ejecutando? ¿qué backend de atajos usa?
voltkit daemon stop
```

El daemon ocupa `io.github.arios4k.Voltkit.Daemon` en el bus de sesión, así que solo se ejecuta uno a la vez.
Asigna los atajos de tu configuración de una de estas formas:

| Escritorio | Cómo se asignan los atajos |
|---|---|
| GNOME 48+, KDE Plasma 5.27+, Hyprland (xdg-desktop-portal ≥ 1.19) | **Portal GlobalShortcuts**: el escritorio te pide aprobarlos una vez |
| GNOME anterior (p. ej. 46) y distribuciones basadas en GNOME | `voltkit shortcuts install` los agrega en *Configuración → Teclado → Atajos personalizados* (`--dry-run` los muestra antes; `shortcuts remove` lo deshace) |
| Sway, river y otros | Asigna `voltkit trigger ocr` / `voltkit trigger color` en la configuración de tu compositor |

`voltkit trigger <acción>` (`ocr`, `color` o `snippets`) envía la acción al daemon en ejecución, o la ejecuta
directamente si no hay daemon. Los resultados y errores aparecen como notificaciones del escritorio.

## Expansión de texto

**Administrar fragmentos:** en la página Expansión de texto, usa **Agregar fragmento**, haz clic en un fragmento
para editarlo, o elimínalo (con Deshacer). Los cambios se guardan en `~/.config/voltkit/config.toml` y llegan de
inmediato a los disparadores al escribir. Si editas ese archivo a mano, ejecuta `voltkit daemon reload`.

**Selector de fragmentos** (sin configuración): presiona `Super+Mayús+E`, escribe para buscar y presiona Enter.
Los fragmentos con parámetros `{arg:NAME}` muestran un pequeño formulario. El resultado se pega en la app que
estabas usando, y después se restaura tu portapapeles. El primer pegado pide permiso de escritorio remoto (así es
como Wayland deja que una app presione Ctrl+V por ti), y el permiso se recuerda.

**Disparadores al escribir** (opcionales): escribe `;date` en cualquier app y se convierte en `2026-10-27`.
Enciende **Disparadores al escribir** en la página Expansión de texto. Wayland oculta las pulsaciones de teclas a
otras apps, así que la primera vez pide tu contraseña para permitir el acceso al teclado, y luego funciona de
inmediato: sin terminal y sin cerrar sesión. Al apagarlos se ofrece quitar el acceso de nuevo. Desde una terminal,
lo mismo es `voltkit expander setup --apply` y `voltkit expander remove-access`.

El acceso al teclado permite que los programas que ejecuta la persona frente a la computadora lean el teclado, no
solo Voltkit. Voltkit solo guarda en memoria los últimos caracteres y nunca registra ni guarda lo que escribes.
Consulta `man voltkit-expander` para más detalles.

Para las terminales, que pegan con Ctrl+Mayús+V, define `paste_shortcut` en la sección `[expander]` de la
configuración.

### Herramientas auxiliares

Voltkit usa herramientas estándar cuando están instaladas, así que no agrega dependencias pesadas propias.
Revisa qué falta, con el comando de instalación para tu distribución, cuando quieras:

```bash
voltkit doctor
```

El centro de control muestra un aviso cuando una función necesita atención.

| Función | Herramientas |
|---|---|
| OCR | `tesseract` (+ paquetes de idioma como `tesseract-ocr-spa`) |
| Captura de región, cuentagotas | el portal Screenshot del escritorio; la captura de región recurre a `grim`+`slurp`, `gnome-screenshot`, `spectacle` o `maim` |
| Portapapeles (daemon, CLI) | `wl-clipboard` (Wayland) o `xclip` (X11). Sin él, el resultado igual se muestra en una notificación |
| Compresión | integrada para PNG; `jpegoptim` (JPEG), `cwebp` (WebP), `ffmpeg` (MP4) |

## Línea de comandos

Cada módulo también funciona sin interfaz, así que puedes asignarlo a un atajo en cualquier escritorio:

```bash
voltkit                       # abre el centro de control
voltkit ocr --lang spa        # selecciona una región → texto en el portapapeles
voltkit pick-color            # cuentagotas → HEX en el portapapeles
voltkit color '#3584e4'       # HEX / RGB / HSL + contraste sobre blanco/negro
voltkit compress *.png --quality 75
voltkit expand 'Hola {arg:who}, {date:%A}' --arg who=Sam
voltkit qr make https://example.com  # código QR en la terminal (o --output code.png)
voltkit qr scan               # lee un código QR en pantalla
voltkit ruler                 # mide en pantalla
voltkit awake 1h30m           # mantiene activa la computadora (on / off / sin argumento para ver el estado)
voltkit using /media/$USER/USB  # ¿qué mantiene ocupada esta unidad? (o un puerto: voltkit using 3000)
voltkit sleep                 # por qué no se suspendió, y qué la despertó
voltkit report                # esta computadora, sus revisiones y cambios recientes (--memory: ranuras y ampliaciones)
voltkit shortcuts who super+v # qué hace un atajo (también: shortcuts list, shortcuts clashes)
voltkit smart-paste           # pega el portapapeles convertido
wl-paste | voltkit convert json  # las conversiones de Pegado inteligente en scripts (voltkit convert las muestra)
```

Los comandos, sus argumentos (`on`, `off`, `json`…) y las claves de configuración son los mismos en todos los
idiomas; la ayuda y los mensajes salen en el tuyo.

La documentación completa está en las páginas del manual: `man voltkit`, `man voltkit-expand`, etc.

### Páginas del manual

Las páginas de [`man/`](man/) se generan a partir de las definiciones de la CLI, así que siempre coinciden con
`--help`; las traducciones están en `man/es/`, `man/de/`, `man/fr/` y `man/zh_CN/`. Instálalas junto al binario, donde `man` las encuentra
automáticamente:

```bash
mkdir -p ~/.cargo/share/man/man1 ~/.cargo/share/man/es/man1
cp man/*.1 ~/.cargo/share/man/man1/ && cp man/es/*.1 ~/.cargo/share/man/es/man1/
```

Después de cambiar un comando o su texto de ayuda, regenéralas; `cargo test` falla hasta que lo hagas:

```bash
cargo run -- manpages man
```

## Configuración

`~/.config/voltkit/config.toml`. Todas las claves son opcionales; consulta
[`data/config.example.toml`](data/config.example.toml).

## Traducciones

Voltkit está traducido al español, al alemán, al francés y al chino simplificado. Los textos están en archivos gettext normales en [`po/`](po/),
que se incluyen en el binario, así que no hay nada extra que instalar. Para agregar o corregir una traducción,
consulta [`po/README.md`](po/README.md).

## Estructura del proyecto

```
src/
  main.rs            punto de entrada de la CLI (clap) → interfaz gráfica sin subcomando
  i18n.rs            traducciones: catálogos .po incluidos, idioma del sistema
  app.rs             centro de control GTK4 / libadwaita
  daemon.rs          servicio en segundo plano: API D-Bus, atajos del portal, inicio automático
  actions.rs         acciones de los atajos (OCR, cuentagotas) con notificaciones
  shortcuts.rs       conversión de aceleradores + alternativa con atajos de GNOME
  picker.rs          ventana emergente del selector de fragmentos
  snippets_ui.rs     página Expansión de texto: lista y editor de fragmentos
  qr_ui.rs           página Código QR
  ruler.rs           Regla de pantalla (capa sobre una captura congelada)
  awake.rs           Mantener activo (portal Inhibit, en manos del daemon)
  awake_ui.rs        página Mantener activo
  system_ui.rs       página Hosts y entorno
  usage_ui.rs        página ¿Qué lo está usando?
  permissions_ui.rs  página Permisos de apps
  sleep_ui.rs        página ¿Por qué no se suspendió?
  gpu_ui.rs          página Ajuste de GPU: lecturas, informe de compatibilidad, controles
  gpu_cli.rs         `voltkit gpu`, el ayudante con permisos de administrador y el servicio en segundo plano
  sysreport_ui.rs    página Informe del sistema
  detective_ui.rs    página Detective de atajos
  smartpaste_ui.rs   ventana y página de Pegado inteligente
  expander.rs        disparadores al escribir (lectura del teclado con evdev, teclado virtual uinput)
  paste.rs           pega en la app enfocada y restaura el portapapeles
  portal.rs          llamadas a los portales XDG (captura, PickColor) y notificaciones
  config.rs          configuración TOML con valores predeterminados
  system.rs          utilidades Wayland/X11: portapapeles, búsqueda de herramientas, ejecución de procesos
  modules/
    ocr.rs           captura de región + Tesseract
    color.rs         lectura, HSL, luminancia y contraste WCAG
    compress.rs      detección de formato + compresión por formato
    snippets.rs      motor de plantillas de fragmentos
    qr.rs            lectura y creación de códigos QR
    hosts.rs         lectura de /etc/hosts que conserva todo lo que no edita
    envvars.rs       variables de entorno de la sesión (environment.d)
    usage.rs         encuentra y cierra los procesos que usan una ruta o un puerto (/proc)
    permissions.rs   el almacén de permisos del portal del escritorio
    sleep.rs         bloqueos de suspensión, historial (journal, UPower) y dispositivos que despiertan
    gpu.rs           detección de GPU, sensores y qué admite cada tarjeta (NVIDIA, AMD, Intel)
    gpu_tune.rs      cambios de GPU validados y transaccionales, curvas de ventilador, protección de inicio
    nvml.rs          enlace con la biblioteca de administración de NVIDIA
    sysreport.rs     hardware, revisiones, salud de unidades, sensores, firmware, instantáneas, memoria SMBIOS
    keybindings.rs   todos los atajos de GSettings: dueños, conflictos, combinaciones libres, cambios
    smartpaste.rs    las conversiones de texto de Pegado inteligente
  manpages.rs        generador de páginas del manual (clap_mangen + secciones escritas a mano)
  screenshots.rs     genera las capturas del README (comando oculto `screenshots`)
po/                  traducciones (español, alemán, francés, chino simplificado) y update.py para actualizarlas
man/                 páginas del manual generadas (man/es, man/de, man/fr, man/zh_CN: traducidas)
packaging/apt/       constructor del repositorio APT y su página de inicio
docs/screenshots/    capturas del README y capture.sh para regenerarlas
docs/ARCHITECTURE.md notas de diseño y planes
```

## Contribuir

```bash
cargo test
cargo clippy
python3 po/update.py --check   # las traducciones están al día y completas
```

Consulta [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) para la API D-Bus y los planes. Está prevista una pantalla de ajustes. Los reportes de errores y las pruebas en otros escritorios son
especialmente bienvenidos.

## Publicar una versión

1. Sube `version` en `Cargo.toml`, ejecuta `cargo run -- manpages man` y haz commit.
2. Crea la etiqueta y súbela: `git tag v0.2.0 && git push origin v0.2.0`.

El [flujo de trabajo Release](.github/workflows/release.yml) compila el `.deb`, publica una versión en GitHub con
él y reconstruye el repositorio APT firmado en GitHub Pages. Firma con la clave del secreto `APT_SIGNING_KEY`
(huella `65E8 4B8D 114C 6147 9CCA 4DDC EC0F 6B66 E480 2360`). Para construir el repositorio localmente:
`APT_KEY_ID=<huella> packaging/apt/build-repo.sh target/debian site`.

## Autor

Creado por **Alexis Rios-Negron**.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/about-dark.png?v=0.11.0">
  <img src="docs/screenshots/about-light.png?v=0.11.0" width="640" alt="La ventana Acerca de de Voltkit con el ícono de la app, la versión y el autor Alexis Rios-Negron">
</picture>

## Apoya a Voltkit

Si Voltkit te ha sido útil, considera [hacer una donación de cualquier cantidad](https://paypal.me/AlexisRios) para ayudar a mantener el proyecto y continuar su desarrollo. Cada aportación se agradece de corazón.

La intención es que Voltkit siga siendo gratuito y accesible para todos. ¡Gracias por usarlo y por cualquier apoyo que puedas ofrecer!

## Licencia

MIT © Alexis Rios-Negron
