<p align="center">
  <img src="docs/assets/brand.svg" alt="Uploadimg Desktop: una imagen, tú decides el siguiente paso" width="960">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/versi%C3%B3n-0.1.0--beta.1-2563eb" alt="Versión 0.1.0-beta.1">
  <img src="https://img.shields.io/badge/macOS-Apple%20Silicon-334155" alt="macOS Apple Silicon">
  <img src="https://img.shields.io/badge/estado-simulaci%C3%B3n%20local-64748b" alt="Simulación local">
</p>

# Tu siguiente imagen, a un clic

Uploadimg Desktop vive en la barra de menús de macOS. Copia una imagen, revisa
su vista previa y decide qué hacer con ella. La detección del portapapeles es
opcional y empieza **desactivada en cada inicio**.

**Esta beta es una simulación local:** Compartir genera un enlace ficticio en
`example.invalid`. No sube imágenes ni crea enlaces públicos accesibles.

## Descargar la beta

[**Descargar v0.1.0-beta.1 para macOS arm64**](https://github.com/SergioGMR/uploadimg-desktop/releases/download/v0.1.0-beta.1/Uploadimg-Desktop-0.1.0-beta.1-macos-arm64.dmg)
· [Notas y SHA256SUMS](https://github.com/SergioGMR/uploadimg-desktop/releases/tag/v0.1.0-beta.1)

1. Descarga el DMG y `SHA256SUMS` desde la misma Release.
2. Comprueba el archivo: `shasum -a 256 Uploadimg-Desktop-0.1.0-beta.1-macos-arm64.dmg`
   y compara el resultado con su línea en `SHA256SUMS`.
3. Abre el DMG y arrastra **Uploadimg Desktop** a **Aplicaciones**.
4. Abre la aplicación y usa su icono de imagen en la barra de menús.

El bundle lleva firma local ad-hoc, sin certificado de distribución Apple ni
notarización. macOS puede bloquear su apertura; si decides continuar tras
verificar origen y checksum, usa la opción de apertura que ofrezca
**Ajustes del Sistema → Privacidad y seguridad**.

## Del portapapeles a la vista previa

| Acción | Qué ocurre |
| --- | --- |
| Generar imagen sintética | Prueba el flujo con una imagen de demostración. |
| Activar detección | Admite nuevas imágenes copiadas y muestra su vista previa. |
| `Control + Command + Shift + 4` | El atajo de macOS copia una captura al portapapeles. |
| Pegar imagen | Admite una imagen mediante una acción explícita. |
| Ignorar / Compartir | Descarta la candidata o crea un resultado simulado. |
| Pausar / Salir | Detiene la detección temporalmente o cierra la aplicación. |

Cerrar la ventana mantiene el proceso en la barra de menús. Las imágenes y los
resultados viven en memoria; se descartan al terminar el proceso. La aplicación
no instala un atajo global ni captura la pantalla por su cuenta.

## Disponibilidad y desarrollo

La evidencia de ejecución cubre **macOS 26.6.2 en Apple Silicon arm64**. La primera
beta distribuye el bundle debug probado. Windows, Linux, macOS Intel, versiones
anteriores de macOS, subidas reales y detección de carpetas quedan pendientes.

El núcleo está escrito en Rust, la integración macOS en Swift y la interfaz local
en HTML/CSS/JavaScript con Tauri. Consulta [la aplicación](desktop/README.md),
[cómo contribuir](CONTRIBUTING.md) y [cómo publicar una versión](docs/releasing.md).
Los documentos de arquitectura/API en `docs/` incluyen propuestas todavía sin implementar.

[Comunica un error o una propuesta](https://github.com/SergioGMR/uploadimg-desktop/issues/new/choose).
Al iniciar trabajo se revisan las issues relacionadas; no hay vigilancia automática.

El código fuente es público. **No se ha elegido una licencia**: su publicación
no concede por sí sola permisos de uso, modificación o redistribución.
