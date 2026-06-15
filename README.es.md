### Refer - Constructor de referencias personalizadas

- - -

El objetivo principal es proporcionar una herramienta local rápida para cualquier persona que necesite no solo almacenar datos, sino realizar cálculos sobre la marcha "sobre el terreno" sin conexión a Internet.

- - -

### Característica principal — Operaciones
Se añaden opcionalmente. Convierten su referencia en una herramienta de cálculo de ingeniería.

Cree una fórmula con variables que pueden ser:
- campos de tipo `number` de la propia referencia,
- valores introducidos manualmente,
- o ambos al mismo tiempo.

Cada operación se aplica a todos los elementos de la referencia.

Implementado usando la crate [exmex](https://docs.rs/exmex/latest/exmex/index.html) — consulte su documentación para conocer la sintaxis de variables, operadores y constantes.

- - -

### Versiones
Android, Linux, Windows — todas las compilaciones en Github.

### Idiomas
13 idiomas. EN y RU — manualmente, el resto generados con LLM: ES, FR, DE, PT, ZH, JA, KO, IT, NL, TR, AR.

- - -

### Ejemplos integrados
6 referencias de ejemplo con operaciones predefinidas. Siéntase libre de modificarlas y editarlas. Se pueden recrear desde el menú "Crear" con un solo clic.

- **Shrinkflation** — compare precios por unidad de peso/volumen
- **Dilution** — calcule proporciones de mezcla de soluciones
- **Ballistics** — calculadora de trayectoria balística
- **Deposit** — calcule el crecimiento del interés compuesto
- **Geometry** — dimensiones de círculo y esfera, introduzca el radio
- **Oscillator** — valor de la onda en el tiempo t — use la pista de tiempo como referencia

- - -

### Almacenamiento
Directorio `~/Documents/refer`, extensión `.refer`. En realidad son bases de datos SQLite. Cree en un dispositivo, transfiera a otro.

Se admiten subcarpetas — los ejemplos integrados se crean en `refer/example`.

La aplicación solo funciona con esta carpeta y no utiliza Internet en absoluto.

- - -

### Importación
Cree una referencia desde cero o importe desde:

- **CSV, TSV** — se prefiere la codificación UTF-8
- **XLS, XLSX, ODT** — se importa la primera hoja. No recomendado para archivos muy grandes: importación lenta, tamaño resultante grande
- **SQLite** — se importa la primera tabla

- - -

### Principios

**Simplicidad.** Todas las alternativas conocidas están sobrecargadas de funciones. Refer trabaja con un elemento específico; la lista completa nunca se muestra — la búsqueda devuelve un máximo de 10 elementos.

**Mínimas restricciones.** Es probable que encuentre errores — vea los registros en el menú "Configuración" o en la ruta que se muestra en la pantalla principal.

**Seguridad.** El cifrado no se ha añadido intencionadamente — utilice las herramientas de su sistema operativo.

- - -

### ⚠ Advertencia para Android
Se requiere el permiso "Acceso a todos los archivos" para trabajar con la carpeta `Documents/refer`.

Actívelo manualmente (depende de su teléfono):
- Configuración → Aplicaciones → Refer → Permisos → Todos los permisos
- Acceso especial → Acceso a todos los archivos → Activar

- - -

[Rust](https://github.com/rust-lang/rust) · [Tauri](https://github.com/tauri-apps/tauri) · [Leptos](https://github.com/leptos-rs/leptos)  
Operaciones — [Exmex](https://docs.rs/exmex/latest/exmex/index.html)

Código abierto (GPL-3.0). Sin anuncios, sin recopilación de datos, sin servicios en la nube.