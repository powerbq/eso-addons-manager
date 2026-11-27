# ESO Addons Manager

[English](README.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · **Español** · [Русский](README.ru.md) · [Українська](README.uk.md)

Un gestor de escritorio para instalar, actualizar y eliminar complementos de The
Elder Scrolls Online — un reemplazo de Minion más cómodo y autónomo. Las
dependencias de un complemento se instalan automáticamente (de vez en cuando el
gestor te pedirá que elijas una entre varias), y las que ya no hacen falta se
eliminan, de modo que tu carpeta AddOns se mantiene limpia: exactamente lo necesario
para que funcionen tus complementos y nada de sobra.

Funciona en Windows, macOS y Linux. La interfaz está disponible en seis idiomas —
inglés, francés, alemán, español, ruso, ucraniano. El idioma se elige automáticamente
según el idioma de tu sistema y se puede cambiar dentro de la aplicación.

> ¿Ya usas [eso-addons-updater](https://github.com/powerbq/eso-addons-updater)? No
> hay nada que cambiar — sigue actualizando e iniciando el gestor, como antes.
>
> Los nuevos usuarios, en cambio, es mejor que descarguen el gestor desde los
> archivos de abajo. Al iniciarse,
> [eso-addons-updater](https://github.com/powerbq/eso-addons-updater) se actualiza a
> sí mismo y al gestor a la última versión y luego lo inicia — y, para actualizarse,
> sobrescribe su propio ejecutable. En una aplicación sin firmar, los antivirus
> pueden considerar sospechoso este comportamiento y bloquear el inicio. El gestor
> instalado desde uno de los archivos de abajo no hace nada de esto: solo te avisa de
> una nueva versión y ofrece abrir la página de descarga, mientras que tú la
> descargas e instalas por tu cuenta. Así que esta es la forma más sencilla de evitar
> problemas con el antivirus — solo quedará el aviso de que la aplicación tiene un
> [editor desconocido](#windows-smartscreen).

## Descarga

Última versión: <https://github.com/powerbq/eso-addons-manager/releases/latest>

Elige un **instalador** para tu sistema (recomendado), o un zip **autónomo**:

| Sistema | Instalador | Autónomo |
|--------|-----------|----------|
| Windows | `…-windows-amd64-setup.exe` | `eso-addons-manager-windows-amd64.zip` |
| macOS (Apple Silicon) | `…-macos-arm64.dmg` | `eso-addons-manager-macos-arm64.zip` |
| Linux | `.deb` / `.rpm` / `.pkg.tar.zst` / `.AppImage` | `eso-addons-manager-linux-amd64.zip` |

> La arquitectura Intel en macOS no es compatible por el momento.

> Puedes verificar cualquier descarga con el archivo `SHA256SUMS` presente en la
> misma versión.

## Instalación

- **Windows:** ejecuta el instalador `-setup.exe` (ofrece instalación por usuario o
  para todos los usuarios). O descomprime el archivo autónomo en cualquier lugar y
  ejecuta `app.exe`.
- **macOS:** abre el `.dmg` y arrastra la aplicación a Aplicaciones. O descomprime
  el archivo autónomo.
- **Linux:** instala el paquete —
  `sudo dpkg -i eso-addons-manager_*.deb` / `sudo rpm -U eso-addons-manager-*.rpm` /
  `sudo pacman -U eso-addons-manager-*.pkg.tar.zst` — o haz que el `.AppImage` sea
  ejecutable y lánzalo, o descomprime el archivo autónomo (concede permiso de
  ejecución al binario `app`).

### Windows SmartScreen

La aplicación no está firmada con un certificado y se distribuye tal cual, por lo
que Windows SmartScreen probablemente avisará de un editor desconocido. Es lo
esperado. Para ejecutarla de todas formas, haz clic en **«Más información»** →
**«Ejecutar de todas formas»**.

## Actualización

El gestor comprueba si hay una versión más reciente al iniciarse y ofrece abrir la
página de descarga. Las actualizaciones nunca se aplican de forma silenciosa —
descargas e instalas tú mismo la nueva versión (vuelve a ejecutar el instalador,
actualiza el paquete con tu gestor de paquetes, o reemplaza los archivos
autónomos). La comprobación se puede desactivar, y ejecutar bajo demanda, desde el
grupo **☰ menú → Actualizaciones**.

## Antes de empezar

> **Importante.** Haz una copia de seguridad de tu carpeta `AddOns` antes del
> primer uso. El gestor elimina de la carpeta seleccionada todo lo que no
> gestiona, así que elige la carpeta con cuidado. Para incorporar los
> complementos que ya tienes, usa **Analizar los complementos existentes** (ver más
> abajo).

## Uso

1. **Elige la carpeta AddOns del juego** — **☰ menú → Carpeta AddOns**. Un cuadro de
   diálogo explica dónde guarda el juego normalmente sus complementos y muestra una
   ruta de ejemplo para tu sistema.

2. **Analizar los complementos existentes** — incorpora los complementos ya
   presentes en la carpeta elegida, para no tener que volver a añadirlos. Puedes
   analizar antes de la primera sincronización o más tarde. Atención: si
   **Sincronizar al iniciar** está activado, la sincronización automática eliminará
   en el próximo inicio los complementos añadidos por medios externos antes de que
   los analices — tenlo en cuenta.

3. **Catálogo** — la pantalla principal de la aplicación, con la lista de todos los
   complementos disponibles. Arriba: búsqueda por nombre, autor o carpeta, un
   **Filtro** (**Instalados**, **Dependencias**, **Favoritos**) y **Ordenar por**
   (nombre, autor, fecha de actualización, número de descargas, etc.). Selecciona un
   complemento para abrir su ficha con su **Descripción**, su **Registro de cambios**
   y capturas de pantalla. Desde aquí puedes instalar o eliminar un complemento,
   añadirlo a **Favoritos** y abrir su página en esoui.com.

4. **Sincronizar** — actualiza todos los complementos instalados y sus bibliotecas,
   y elimina lo sobrante. Activa **Sincronizar al iniciar** (abajo a la derecha) para
   hacerlo automáticamente al arrancar.

5. **Dependencias** — se gestionan automáticamente: las bibliotecas se instalan,
   actualizan y eliminan cuando ningún complemento las necesita ya. Solo mantienes
   la lista de complementos que quieres. Cuando varios complementos proporcionan la
   misma biblioteca, el gestor normalmente lo resuelve por su cuenta (prefiere
   aquella a la que enlazan los autores del complemento). Solo cuando realmente no
   puede decidir **te pide elegir**. Tu elección se recuerda, y cada una de esas
   bibliotecas (elegida por ti o automáticamente) se muestra en la pestaña
   **Bibliotecas**, donde puedes cambiarla o restablecerla a automático en cualquier
   momento.

6. **Tamriel Trade Centre** — si TTC está instalado, el gestor actualiza sus tablas
   de precios automáticamente durante una sincronización, sin el cliente de TTC. El
   interruptor y la región (**Europa / América**) están en el grupo **☰ menú →
   Precios TTC**.

7. **Complementos obsoletos** — por defecto el catálogo muestra solo los
   complementos actuales. Si necesitas un complemento antiguo, descontinuado y por
   eso oculto, activa **☰ menú → Experimental → Complementos obsoletos** y aparecerá
   en el catálogo. Es experimental, así que está desactivado por defecto.

## Avanzado

- **Pestaña Exclusiones.** Protege archivos frente a la eliminación — por ejemplo
  las tablas de precios actualizables de Tamriel Trade Centre o la base descargada de
  las coordenadas de todos los puntos de recolección de HarvestMap-Data. Las
  exclusiones automáticas para **Tamriel Trade Centre** y **HarvestMap-Data** vienen
  integradas y se activan cuando esos complementos están instalados; puedes añadir
  tus propias reglas a la izquierda.

- **Pestaña Parches.** Ajusta el *contenido* de los archivos de un complemento
  durante una sincronización — un pequeño buscar y reemplazar dentro de un archivo —
  para que una corrección que necesitas no sea sobrescrita por los datos originales
  del complemento. Los parches integrados se aplican automáticamente cuando el
  complemento correspondiente está instalado y se muestran en solo lectura,
  etiquetados por complemento (por ejemplo, para mantener viva la integración con
  Tamriel Trade Centre de **DovahMova**). Puedes añadir tus propias reglas a la
  izquierda, cambiar el orden en que se aplican y activarlas o desactivarlas
  individualmente. Esta es la solución autoservicio cuando un parche integrado deja
  de coincidir tras una actualización de un complemento, o cuando un complemento que
  necesitas aún no tiene un parche integrado.

> Las reglas de exclusiones y parches se definen con expresiones regulares;
> escríbelas y pruébalas en <https://regex101.com/>.

> Si un complemento necesita exclusiones o parches para funcionar correctamente, deja
> tu comentario y su soporte podrá añadirse a las listas integradas.

- **Tus propios complementos (custom).** Para instalar complementos que no están en
  el catálogo de ESO UI — los tuyos, o los que solo tienes como `.zip` — coloca el
  archivo en la carpeta **custom** (**☰ menú → Carpetas de la aplicación →
  Complementos personalizados**). En cada sincronización el gestor extrae cada
  `custom/*.zip` en tu carpeta AddOns y lo mantiene ahí junto a los complementos del
  catálogo, para que no se trate como un archivo extraño y se elimine. Sus
  dependencias se resuelven como las de cualquier otro complemento.

- **Dónde se guardan tus datos.** El gestor almacena sus datos en las ubicaciones
  estándar del sistema operativo, propias de cada usuario, y no junto al ejecutable;
  cada carpeta se puede abrir directamente desde el menú:
  - **☰ menú → Carpetas de la aplicación → Configuración** — los ajustes `app.ini`
    (complementos instalados, favoritos, elecciones de bibliotecas, exclusiones,
    parches, opciones) y sus copias de seguridad;
  - **☰ menú → Carpetas de la aplicación → Caché de la aplicación** — descargas en
    caché: archivos de complementos, catálogo y datos de precios de TTC;
  - **☰ menú → Carpetas de la aplicación → Complementos personalizados** — tus propios
    archivos `.zip`, extraídos en la carpeta AddOns en cada sincronización;
  - **☰ menú → Registro → Abrir carpeta** — registros guardados.

  Las carpetas siguen las convenciones de cada plataforma
  (`%LOCALAPPDATA%\EsoAddonsManager` en Windows,
  `~/Library/…/io.github.powerbq.eso-addons-manager` en macOS, los directorios base
  XDG bajo `eso-addons-manager` en Linux). En el primer inicio, los datos de la
  antigua disposición (en el directorio de trabajo) se migran automáticamente a estas
  ubicaciones.

## Si algo va mal

Si la aplicación no se comporta como esperas, usa **☰ menú → Soporte → Informar de
un problema**. En ese cuadro de diálogo puedes elegir qué adjuntar al informe,
describir qué pasó e indicar cómo contactarte — y enviarlo todo al desarrollador.

## Compilación

La compilación desde el código fuente, el empaquetado y la estructura de CI
se describen en [BUILD.md](BUILD.md) (en inglés).

## Licencia

Publicado bajo la [Licencia MIT](LICENSE).

Copyright (c) 2026 powerbq.

Eres libre de bifurcar, modificar y redistribuir este software, pero el aviso de
copyright original y el texto de la licencia MIT deben conservarse en todas las
copias y obras derivadas.
