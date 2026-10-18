# Kimün (km)

Una herramienta de línea de comandos rápida para analizar código, escrita en Rust. Ejecuta `km score` en cualquier proyecto para obtener una nota general de salud (de A++ a F--) en cinco dimensiones de calidad (complejidad cognitiva, duplicación, profundidad de indentación, esfuerzo de Halstead y tamaño de archivo), junto con la lista de los archivos que más atención necesitan.

```
$ km score
Code Health Score
──────────────────────────────────────────────────────────────────
 Project Score:  A (91.2)
 Files Analyzed: 142
 Total LOC:      19,084
──────────────────────────────────────────────────────────────────
 Dimension                 Weight   Score   Grade
──────────────────────────────────────────────────────────────────
 Cognitive Complexity         32%    84.1   B+
 Duplication                  21%   100.0   A++
 Indentation Complexity       16%    93.2   A+
 Halstead Effort              21%    87.5   A-
 File Size                    10%   100.0   A++
──────────────────────────────────────────────────────────────────

 Files Needing Attention (worst per-file scores, excl. duplication)
──────────────────────────────────────────────────────────────────
 Score  Grade  File                       Issues
──────────────────────────────────────────────────────────────────
  57.7  D-     ./src/deps/analyzer.rs     Cognitive: 36
  61.0  D      ./src/cogcom/analyzer.rs   Cognitive: 35
  70.3  C      ./src/smells/rules.rs      Cognitive: 20
  71.6  C      ./src/hal/tokenizer.rs     Cognitive: 22
  71.8  C      ./src/loc/mod.rs           Cognitive: 19
  72.8  C      ./src/knowledge/mod.rs     Cognitive: 21
  73.8  C+     ./src/dups/mod.rs          Cognitive: 19
  75.0  C+     ./src/cli_help.rs          Cognitive: 17
  75.6  C+     ./src/test_blocks.rs       Cognitive: 17
  77.3  B-     ./src/walk.rs              Cognitive: 16
──────────────────────────────────────────────────────────────────
```

Eso es Kimün midiendo su propio código: 140 archivos y 18.963 líneas de código, sin los tests, en 0,11 segundos en un Apple M3 Max. Contar los 235 archivos de `src/`, con los tests, le toma 0,04 segundos a `km loc` y 2,4 a `cloc`.

**Sitio web: [kimun.tools](https://kimun.tools/)** · [Un video de dos minutos](https://kimun.tools/#video) con la nota, las métricas, lo que git recuerda, el impacto de un cambio y la compuerta para CI · También [en inglés](README.md).

Además del puntaje agregado, Kimün ofrece 18 comandos especializados:

- **Métricas estáticas**: líneas de código por lenguaje (clasificadas como las clasifica [cloc](https://github.com/AlDanial/cloc)), detección de código duplicado (Regla de Tres), complejidad de Halstead, complejidad ciclomática, complejidad cognitiva (SonarSource), complejidad por indentación, dos variantes del índice de mantenibilidad (Visual Studio y verifysoft), detección de code smells, dependencias entre archivos y sus ciclos, y un informe completo con todas las métricas.
- **Análisis basado en git**: detección de hotspots (frecuencia de cambio × complejidad, método de Thornhill), churn (frecuencia de cambio pura), propiedad del código y mapas de conocimiento con `git blame`, acoplamiento temporal entre archivos que cambian juntos, impacto de un diff (difusión y cambios conjuntos que faltan), resumen de propiedad por autor y clasificación de archivos por antigüedad (Active / Stale / Frozen).
- **Para agentes de código**: todos los comandos que miden pueden responder en JSON, y `km ai` es una integración opcional con Claude que ejecuta las herramientas y escribe un informe narrativo. Ningún otro comando llama a un modelo.

> Aviso: este repositorio no es la aplicación de notas para la consola `kimun`, escrita en Rust por nico2sh. Ese proyecto está en https://github.com/nico2sh/kimun.

## Instalación

```bash
cargo install kimun              # desde crates.io
brew install lnds/kimun/kimun    # con Homebrew, en macOS y Linux
```

Cualquiera de los dos instala el binario `km`. También hay binarios listos para macOS (Apple Silicon), Linux (x86_64 y arm64) y Windows (x86_64) en la [página de releases](https://github.com/lnds/kimun/releases). Desde una copia de este repositorio, `cargo install --path .` lo compila desde el código fuente.

### Autocompletado para el shell

Genera e instala un script de autocompletado para tu shell:

```bash
# zsh
km completions zsh > ~/.zfunc/_km
# add to ~/.zshrc if not already present:
#   fpath=(~/.zfunc $fpath)
#   autoload -Uz compinit && compinit

# bash
km completions bash > /etc/bash_completion.d/km

# fish
km completions fish > ~/.config/fish/completions/km.fish
```

## Comandos

### `km loc` -- Contar líneas de código

```bash
km loc [path]
```

Ejecútalo en el directorio actual:

```bash
km loc
```

Ejecútalo en una ruta específica:

```bash
km loc src/
```

Los archivos de tests quedan fuera por defecto, así que los totales son menores que los de `cloc` sobre el mismo directorio. Con `--include-tests` ambos cuentan los mismos archivos.

Opciones:

| Flag | Descripción |
|------|-------------|
| `-v`, `--verbose` | Muestra estadísticas de resumen (archivos leídos, únicos, ignorados, tiempo transcurrido) |
| `--by-author` | Desglosa las líneas de código por autor de git (requiere un repositorio git) |
| `--include-tests` | Cuenta también los archivos y directorios de tests (por defecto quedan fuera) |
| `--format {table,json,short,terse}` | Formato de salida (por defecto: table) |

Ejemplo de salida:

```
────────────────────────────────────────────────────────────────────
 Language                Files        Blank      Comment         Code
────────────────────────────────────────────────────────────────────
 Rust                        5          120           45          850
 TOML                        1            2            0           15
────────────────────────────────────────────────────────────────────
 SUM:                        6          122           45          865
────────────────────────────────────────────────────────────────────
```

### `km dups` -- Detectar código duplicado

Encuentra bloques de código duplicado entre archivos con una ventana deslizante. Aplica la **Regla de Tres**: los duplicados que aparecen 3 veces o más se marcan como **CRITICAL** (se recomienda refactorizar), y los que aparecen dos veces como **TOLERABLE**.

Los archivos y directorios de tests se excluyen por defecto, porque los tests suelen contener repetición intencional.

```bash
km dups [path]
```

Opciones:

| Flag | Descripción |
|------|-------------|
| `-r`, `--report` | Muestra un informe detallado con la ubicación de los duplicados y muestras de código |
| `--show-all` | Muestra todos los grupos de duplicados (por defecto: los 20 primeros) |
| `--min-lines N` | Mínimo de líneas para un bloque duplicado (por defecto: 6) |
| `--include-tests` | Incluye los tests, ejemplos y benchmarks en el análisis (excluidos por defecto) |
| `--max-duplicates N` | Termina con código 1 si los grupos de duplicados superan este límite (`--max-duplicates 0` falla con cualquier duplicado) |
| `--max-dup-ratio PERCENT` | Termina con código 1 si la proporción de líneas duplicadas supera este porcentaje (p. ej. `--max-dup-ratio 5.0`) |
| `--fail-on-increase REF` | Termina con código 1 si la proporción de duplicación actual es mayor que en la ref de git indicada (p. ej. `origin/main`). Evita que la deuda crezca en silencio en CI |
| `--format {table,json,short,terse}` | Formato de salida (por defecto: table) |

Ejemplo de salida resumida:

```
────────────────────────────────────────────────────────────────────
 Duplication Analysis

 Total code lines:                                             3247
 Duplicated lines:                                              156
 Duplication:                                                  4.8%

 Duplicate groups:                                               12
 Files with duplicates:                                           8
 Largest duplicate:                                        18 lines

 Rule of Three Analysis:
   Critical duplicates (3+):     7 groups,    96 lines
   Tolerable duplicates (2x):    5 groups,    60 lines

 Assessment:                                                    Good
────────────────────────────────────────────────────────────────────
```

Ejemplo de salida detallada (`--report`):

```
────────────────────────────────────────────────────────────────────
 [1] CRITICAL: 18 lines, 3 occurrences (36 duplicated lines)

   src/parser.rs:45-62
   src/formatter.rs:120-137
   src/validator.rs:89-106

 Sample:
   fn process_tokens(input: &str) -> Vec<Token> {
       let mut tokens = Vec::new();
       for line in input.lines() {
       ...

────────────────────────────────────────────────────────────────────
 [2] TOLERABLE: 12 lines, 2 occurrences (12 duplicated lines)

   src/main.rs:100-111
   src/cli.rs:200-211

 Sample:
   match result {
       Ok(value) => {
       ...
────────────────────────────────────────────────────────────────────
```

#### Patrones de test excluidos

Por defecto, `km dups` omite los archivos que siguen las convenciones habituales de test:

- **Directorios**: `tests/`, `test/`, `__tests__/`, `spec/`, y los proyectos de prueba de .NET, con el nombre de lo que prueban (`App.Tests/`, `Orders.UnitTests/`, `Api.IntegrationTest/`)
- **Ejemplos y benchmarks**: `examples/`, `example/`, `benches/`, `bench/`, `benchmarks/`, `benchsuite/`, código que se ejecuta para mostrar o medir el proyecto y no se distribuye con él
- **Por extensión**: `*_test.rs`, `*_test.go`, `test_*.py`, `*.test.js`, `*.spec.ts`, `*Test.java`, `*_test.cpp` y más
- **Módulos de test de Rust**: un archivo declarado bajo `#[cfg(test)]` sin cuerpo (`#[cfg(test)] mod testutil;`, con o sin `#[path]`), y los módulos que están debajo de él

Las mismas reglas valen para todos los comandos. Usa `--include-tests` para analizarlos también, y `--list-excluded` para ver qué archivos quedan fuera y por qué.

#### Tests en línea

Rust guarda los tests dentro del archivo fuente, bajo `#[cfg(test)]`. Todo comando que mide código deja esos ítems fuera junto con los archivos de test: `dups`, `indent`, `hal`, `cycom`, `cogcom`, `mi`, `miv`, `smells`, `hotspots`, `score` y las métricas de `report`. Solo se omite el ítem bajo el atributo, así que el código que sigue a un módulo de tests sí se mide. `--include-tests` los trae de vuelta.

`km loc` y los conteos de líneas de `km report` los siguen contando: son líneas del archivo, tal como las cuenta `cloc`.

#### Archivos generados, datos y prosa

Los comandos que miden código (`dups`, `indent`, `hal`, `cycom`, `cogcom`, `mi`, `miv`, `smells`, `score`, `report`, `init`, `hotspots`, `churn`, `tc`) dejan fuera:

- **Archivos generados**, por nombre (archivos lock, `*.min.js`, `*.bundle.js`, `*.pb.go`, `*_pb2.py`, `*.generated.*`) o por contenido:
  - un comentario en las primeras 20 líneas que dice que el archivo es generado y no se debe editar (el `// Code generated ... DO NOT EDIT.` de Go, protoc);
  - `@generated`, o el `<auto-generated` de C#;
  - el encabezado de Bison o de Flex, o las funciones auxiliares que esbuild pone al inicio de un bundle;
  - la salida de un bundler: un comentario `sourceMappingURL` al final del archivo, o un `<nombre>.map` al lado.
- **Datos y prosa**: JSON, YAML, TOML, XML, Markdown, Text, Properties y archivos PO. Su indentación es estructura, no anidamiento, y cambian en cada versión sin ser código.

`km knowledge` y `km authors` dejan fuera los archivos generados. `km loc` y `km deps` leen todos los archivos: las líneas son líneas, y el código generado igual se importa.

### `km indent` -- Complejidad por indentación

Mide la complejidad de cada archivo a partir de su indentación: la desviación estándar de las profundidades de indentación y la profundidad máxima. Una desviación estándar más alta sugiere un flujo de control más complejo.

```bash
km indent [path]
```

La profundidad se lee como estructura, no como diagramación. Una línea que continúa una instrucción queda a lo más un nivel más adentro que la línea que continúa: los argumentos alineados bajo un paréntesis de apertura, una línea que sigue a otra terminada en `::`, `&&`, `||`, `|>`, `|` o `\`, y una línea que empieza con `.`, `|`, `&&` o `||`. Dentro de un bloque abierto entre paréntesis (el cuerpo de una closure o de un `fn` de Elixir), las profundidades vuelven a contar desde ese bloque. Un archivo con menos de 10 líneas de código no se clasifica (`-`): la dispersión de unas pocas líneas no dice nada. Los archivos de datos y prosa quedan fuera.

Opciones:

| Flag | Descripción |
|------|-------------|
| `--format {table,json,short,terse}` | Formato de salida (por defecto: table) |
| `--include-tests` | Incluye los tests, ejemplos y benchmarks en el análisis (excluidos por defecto) |

### `km hal` -- Métricas de complejidad de Halstead

Calcula las [métricas de complejidad de Halstead](https://en.wikipedia.org/wiki/Halstead_complexity_measures) por archivo, extrayendo operadores y operandos del código fuente.

```bash
km hal [path]
```

#### Métricas

| Símbolo | Métrica | Fórmula | Descripción |
|--------|--------|---------|-------------|
| n1 | Operadores distintos | -- | Operadores únicos en el código |
| n2 | Operandos distintos | -- | Operandos únicos en el código |
| N1 | Total de operadores | -- | Apariciones totales de operadores |
| N2 | Total de operandos | -- | Apariciones totales de operandos |
| n | Vocabulario | n1 + n2 | Tamaño del "alfabeto" usado |
| N | Longitud | N1 + N2 | Número total de tokens |
| V | Volumen | N * log2(n) | Tamaño de la implementación |
| D | Dificultad | (n1/2) * (N2/n2) | Propensión a errores |
| E | Esfuerzo | D * V | Esfuerzo mental para desarrollarlo |
| B | Bugs | V / 3000 | Bugs entregados estimados |
| T | Tiempo | E / 18 segundos | Tiempo de desarrollo estimado |

Un esfuerzo, un volumen y una cantidad de bugs más altos indican código más complejo y más propenso a errores.

Opciones:

| Flag | Descripción |
|------|-------------|
| `--format {table,json,short,terse}` | Formato de salida (por defecto: table) |
| `--include-tests` | Incluye los tests, ejemplos y benchmarks en el análisis (excluidos por defecto) |
| `--top N` | Muestra solo los N primeros archivos (por defecto: 20) |
| `--sort-by METRIC` | Ordena por `effort`, `volume` o `bugs` (por defecto: `effort`) |

Ejemplo de salida:

```
Halstead Complexity Metrics
──────────────────────────────────────────────────────────────────────────────
 File                      n1   n2    N1    N2    Volume     Effort   Bugs
──────────────────────────────────────────────────────────────────────────────
 src/loc/counter.rs       139  116  3130  1169   34367.7   24070888  11.46
 src/main.rs               37   43   520   185    4457.0     354743   1.49
──────────────────────────────────────────────────────────────────────────────
 Total (2 files)                     3650  1354   38824.7   24425631  12.95
```

#### Lenguajes soportados

Rust, Python, JavaScript, JSX, TypeScript, Go, C, C++, C#, Java, Objective-C, PHP, Dart, Ruby, Kotlin, Swift, Shell (Bash/Zsh), Elixir, Kaikai, Fortran, Pascal (Delphi).

### `km cycom` -- Complejidad ciclomática

Calcula la complejidad ciclomática por archivo y por función contando los puntos de decisión (`if`, `for`, `while`, `match`, `&&`, `||`, etc.).

```bash
km cycom [path]
```

Opciones:

| Flag | Descripción |
|------|-------------|
| `--format {table,json,short,terse,github,codeclimate}` | Formato de salida (por defecto: table). `github` emite anotaciones de GitHub Actions; `codeclimate` (alias: `gitlab`) emite JSON de CodeClimate para GitLab Code Quality |
| `--include-tests` | Incluye los tests, ejemplos y benchmarks en el análisis (excluidos por defecto) |
| `--top N` | Muestra solo los N primeros archivos (por defecto: 20) |
| `--min-complexity N` | Omite los archivos cuya función más compleja está por debajo de N; con `--per-function`, oculta también las funciones por debajo de N (por defecto: 1) |
| `--per-function` | Muestra el desglose por función |

#### Cómo se encuentran las funciones

`cycom`, `cogcom` y `smells` comparten un mismo detector de funciones. Una declaración se reconoce por el marcador de su lenguaje (`fn`, `def`, `func`, `function`, `defmacro`) o, en la familia C, por su forma, sobre la firma completa aunque ocupe varias líneas:

- parámetros en varias líneas, el nombre solo en la línea de arriba (estilo GNU), la `{` en la línea siguiente;
- una línea que empieza dentro de paréntesis abiertos (la continuación de una condición, un parámetro con atributo) nunca es una declaración, ni tampoco una anotación (`@Override`, `[HttpGet]`) o una clase anónima (`new Adapter() {`);
- las directivas de preprocesador y las líneas que un `\` final les une no son código: no declaran funciones ni suman complejidad;
- los métodos de Go se nombran sin su receptor;
- en JavaScript y TypeScript, las funciones asignadas a un nombre (`res.send = function`, `const f = (x) => {`, `name: Type = <T>(x) => {`) toman ese nombre, una función pasada como argumento es `<anonymous>`, y las llaves de una expresión regular literal no son bloques;
- en Elixir, el código dentro de un bloque `quote` no es parte de la función que lo contiene, y las funciones que declara se miden por separado.

### `km cogcom` -- Complejidad cognitiva

Calcula la complejidad cognitiva por archivo y por función con el [método de SonarSource](https://www.sonarsource.com/docs/CognitiveComplexity.pdf) (2017). A diferencia de la complejidad ciclomática, la complejidad cognitiva mide qué tan difícil es *entender* el código: penaliza las estructuras muy anidadas y premia el flujo de control lineal.

```bash
km cogcom [path]
```

Opciones:

| Flag | Descripción |
|------|-------------|
| `--format {table,json,short,terse,github,codeclimate}` | Formato de salida (por defecto: table). `github` emite anotaciones de GitHub Actions; `codeclimate` (alias: `gitlab`) emite JSON de CodeClimate para GitLab Code Quality |
| `--include-tests` | Incluye los tests, ejemplos y benchmarks en el análisis (excluidos por defecto) |
| `--top N` | Muestra solo los N primeros archivos (por defecto: 20) |
| `--min-complexity N` | Omite los archivos cuya función más compleja está por debajo de N; con `--per-function`, oculta también las funciones por debajo de N (por defecto: 1) |
| `--per-function` | Muestra el desglose por función |
| `--sort-by METRIC` | Ordena por `total`, `max` o `avg` (por defecto: `total`) |

En los lenguajes de llaves una función termina donde sus llaves se balancean. Un cuerpo introducido por `=` (`fn f(x) = expr`, como en Kaikai, Kotlin o Scala) no tiene una llave de cierre propia: sigue mientras sus llaves estén abiertas y las líneas siguientes tengan más indentación que la declaración. `km cycom` encuentra las funciones de la misma manera.

Las palabras clave y las llaves se leen en orden a lo largo de cada línea, así que el código escrito en una línea obtiene el mismo puntaje que en varias: `if (a) { return 1 } else { return 2 }` es 2, y `for (v of xs) { if (y) { return v } }` es 3. El bloque de una palabra clave de control se abre en la siguiente `{`, en su misma línea o, en estilo Allman, en la siguiente; `else if` anida su bloque igual que `if` y `else`.

En Elixir, `with`, `unless` y `receive` son incrementos estructurales como `case`, y `and`, `or`, `&&` y `||` cuentan como secuencias booleanas. Para `km cycom`, cada cláusula (`->`) de un `case`, `cond`, `receive` o `fn` de varias cláusulas es una rama, igual que cada etiqueta `case` de C, y `with`, `unless`, `and`, `or`, `&&` y `||` suman uno cada uno.

### `km mi` -- Índice de mantenibilidad (variante de Visual Studio)

Calcula el [índice de mantenibilidad](https://learn.microsoft.com/en-us/visualstudio/code-quality/code-metrics-maintainability-index-range-and-meaning) por archivo con la fórmula de Visual Studio. El MI se normaliza a una escala de 0 a 100, sin el término de peso de los comentarios.

```bash
km mi [path]
```

#### Fórmula

```
MI = MAX(0, (171 - 5.2 * ln(V) - 0.23 * G - 16.2 * ln(LOC)) * 100 / 171)
```

Donde V = volumen de Halstead, G = complejidad ciclomática, LOC = líneas de código.

#### Umbrales

| Puntaje MI | Nivel | Significado |
|----------|-------|---------|
| 20–100 | green | Buena mantenibilidad |
| 10–19 | yellow | Mantenibilidad moderada |
| 0–9 | red | Baja mantenibilidad |

Opciones:

| Flag | Descripción |
|------|-------------|
| `--format {table,json,short,terse}` | Formato de salida (por defecto: table) |
| `--include-tests` | Incluye los tests, ejemplos y benchmarks en el análisis (excluidos por defecto) |
| `--top N` | Muestra solo los N primeros archivos (por defecto: 20) |
| `--sort-by METRIC` | Ordena por `mi` (ascendente), `volume`, `complexity` o `loc` (por defecto: `mi`) |

Ejemplo de salida:

```
Maintainability Index (Visual Studio)
──────────────────────────────────────────────────────────────────────
 File                       Volume Cyclo   LOC     MI  Level
──────────────────────────────────────────────────────────────────────
 src/loc/counter.rs        32101.6   115   731    0.0  red
 src/main.rs               11189.6    16   241   17.5  yellow
 src/loc/report.rs          6257.0    13   185   22.2  green
──────────────────────────────────────────────────────────────────────
 Total (3 files)                         1157   13.2
```

### `km miv` -- Índice de mantenibilidad (variante de verifysoft)

Calcula el [índice de mantenibilidad](https://www.verifysoft.com/en_maintainability.html) por archivo. El MI combina el volumen de Halstead, la complejidad ciclomática, las líneas de código y la proporción de comentarios en un solo puntaje de mantenibilidad.

Esta es la variante de verifysoft.com, que incluye un término de peso de los comentarios (MIcw) que premia el código bien comentado.

```bash
km miv [path]
```

#### Fórmula

```
MIwoc = 171 - 5.2 * ln(V) - 0.23 * G - 16.2 * ln(LOC)
MIcw  = 50 * sin(sqrt(2.46 * radians(PerCM)))
MI    = MIwoc + MIcw
```

Donde V = volumen de Halstead, G = complejidad ciclomática, LOC = líneas de código, PerCM = porcentaje de comentarios (convertido a radianes).

#### Umbrales

| Puntaje MI | Nivel | Significado |
|----------|-------|---------|
| 85+ | good | Fácil de mantener |
| 65–84 | moderate | Mantenibilidad razonable |
| <65 | difficult | Difícil de mantener |

Opciones:

| Flag | Descripción |
|------|-------------|
| `--format {table,json,short,terse}` | Formato de salida (por defecto: table) |
| `--include-tests` | Incluye los tests, ejemplos y benchmarks en el análisis (excluidos por defecto) |
| `--top N` | Muestra solo los N primeros archivos (por defecto: 20) |
| `--sort-by METRIC` | Ordena por `mi` (ascendente), `volume`, `complexity` o `loc` (por defecto: `mi`) |

Ejemplo de salida:

```
Maintainability Index
────────────────────────────────────────────────────────────────────────────────
 File                       Volume Cyclo   LOC  Cmt%   MIwoc      MI  Level
────────────────────────────────────────────────────────────────────────────────
 src/loc/counter.rs        32101.6   115   731   3.6   -16.2     2.8  difficult
 src/main.rs                8686.7    14   204  14.6    34.5    68.2  moderate
 src/util.rs                2816.9    18    76   9.5    55.4    84.7  moderate
────────────────────────────────────────────────────────────────────────────────
 Total (3 files)                         1011                  51.9
```

### `km hotspots` -- Análisis de hotspots

Encuentra hotspots: archivos que cambian con frecuencia Y además tienen alta complejidad. Se basa en el método de Adam Thornhill ("Your Code as a Crime Scene").

```bash
km hotspots [path]
```

#### Fórmula

```
Score = Commits × Complexity
```

Los archivos con puntaje alto concentran el riesgo: cambian mucho y son complejos, así que son los candidatos más valiosos para refactorizar.

Por defecto, la complejidad se mide con la **indentación total** (la suma de los niveles lógicos de indentación de todas las líneas de código), siguiendo el método original de Thornhill en "Your Code as a Crime Scene". Usa `--complexity cycom` para medirla con la complejidad ciclomática.

Requiere un repositorio git. Los commits de merge se excluyen de la cuenta.

Opciones:

| Flag | Descripción |
|------|-------------|
| `--format {table,json,short,terse}` | Formato de salida (por defecto: table) |
| `--include-tests` | Incluye los tests, ejemplos y benchmarks en el análisis (excluidos por defecto) |
| `--top N` | Muestra solo los N primeros archivos (por defecto: 20) |
| `--sort-by METRIC` | Ordena por `score`, `commits` o `complexity` (por defecto: `score`) |
| `--since DURATION` | Considera solo los commits desde este momento (p. ej. `30d`, `6m`, `1y`) |
| `--complexity METRIC` | `indent` (por defecto, Thornhill) o `cycom` (ciclomática) |

Unidades de duración: `d` (días), `m` (meses, aprox. 30 días), `y` (años, aprox. 365 días).

Ejemplo de salida (por defecto, complejidad por indentación):

```
Hotspots (Commits × Total Indent Complexity)
──────────────────────────────────────────────────────────────────────────────
 File                    Language Commits Total Indent      Score
──────────────────────────────────────────────────────────────────────────────
 src/main.rs                 Rust      18        613      11034
 src/loc/counter.rs          Rust       7       1490      10430
 src/dups/detector.rs        Rust       7       1288       9016
 src/dups/mod.rs             Rust       9        603       5427
 src/report/mod.rs           Rust       4        998       3992
──────────────────────────────────────────────────────────────────────────────

Score = Commits × Total Indentation (Thornhill method).
High-score files are change-prone and complex — prime refactoring targets.
```

Ejemplo de salida (`--complexity cycom`):

```
Hotspots (Commits × Cyclomatic Complexity)
──────────────────────────────────────────────────────────────────────────────
 File                     Language Commits Cyclomatic      Score
──────────────────────────────────────────────────────────────────────────────
 src/loc/counter.rs           Rust       7        115        805
 src/dups/mod.rs              Rust       9         44        396
 src/main.rs                  Rust      18         21        378
 src/cycom/analyzer.rs        Rust       4         92        368
 src/dups/detector.rs         Rust       7         46        322
──────────────────────────────────────────────────────────────────────────────

Score = Commits × Cyclomatic Complexity.
High-score files are change-prone and complex — prime refactoring targets.
```

### `km knowledge` -- Análisis de propiedad del código

Analiza los patrones de propiedad del código con git blame (mapas de conocimiento). Se basa en el método de Adam Thornhill ("Your Code as a Crime Scene", capítulos 8 y 9).

```bash
km knowledge [path]
```

Identifica el riesgo de bus factor y la concentración del conocimiento por archivo. Los archivos generados (archivos lock, JS minificado, archivos que dicen ser generados, ver [Archivos generados, datos y prosa](#archivos-generados-datos-y-prosa)) se excluyen automáticamente.

#### Niveles de riesgo

| Riesgo | Condición | Significado |
|------|-----------|---------|
| CRITICAL | 1 persona es dueña de más del 80% | Alto riesgo de bus factor |
| HIGH | 1 persona es dueña del 60-80% | Concentración significativa |
| MEDIUM | 2-3 personas son dueñas de más del 80% entre todas | Concentración moderada |
| LOW | Bien distribuido | Propiedad sana |

#### Detección de pérdida de conocimiento

Usa `--since` para definir qué es "actividad reciente". Si el dueño principal de un archivo no tiene commits en ese período, el archivo se marca con riesgo de **pérdida de conocimiento**. Usa `--risk-only` para mostrar solo esos archivos.

Opciones:

| Flag | Descripción |
|------|-------------|
| `--format {table,json,short,terse}` | Formato de salida (por defecto: table) |
| `--include-tests` | Incluye los tests, ejemplos y benchmarks en el análisis (excluidos por defecto) |
| `--top N` | Muestra solo los N primeros archivos (por defecto: 20) |
| `--sort-by METRIC` | Ordena por `concentration`, `diffusion` o `risk` (por defecto: `concentration`) |
| `--since DURATION` | Define la ventana de actividad reciente para la pérdida de conocimiento (p. ej. `6m`, `1y`, `30d`) |
| `--risk-only` | Muestra solo los archivos con riesgo de pérdida de conocimiento |
| `--summary` | Agrupa por autor: archivos de los que es dueño, líneas, lenguajes, peor riesgo |
| `--bus-factor` | Muestra el bus factor del proyecto: cuántos colaboradores tienen que irse para que la mayor parte del código quede sin autor |
| `--bus-factor-threshold PCT` | Porcentaje del código, en líneas, hasta el que cuenta el bus factor (por defecto: 50) |
| `--author NAME` | Muestra solo los archivos cuyo dueño es este autor (busca la subcadena sin distinguir mayúsculas) |

Ejemplo de salida:

```
Knowledge Map — Code Ownership
──────────────────────────────────────────────────────────────────────────────
 File                       Language  Lines  Owner         Own%  Contrib  Risk
──────────────────────────────────────────────────────────────────────────────
 src/loc/counter.rs             Rust    731  E. Diaz        94%        2  CRITICAL
 src/main.rs                    Rust    241  E. Diaz        78%        3  HIGH
 src/walk.rs                    Rust    145  E. Diaz        55%        5  MEDIUM
──────────────────────────────────────────────────────────────────────────────

Files with knowledge loss risk (primary owner inactive): 1
  src/legacy.rs (Former Dev)
```

Usa `--bus-factor` para calcular cuántos colaboradores tienen que irse para que la mayor parte del código quede sin nadie que lo haya escrito (el *truck factor* de Avelino et al., "A novel approach for estimating truck factors", ICPC 2016):

- Los **autores de un archivo** son aquellos a quienes blame les da al menos el 75% de las líneas de su autor principal. Un colaborador con unas pocas líneas en un archivo no es uno de sus autores.
- Los colaboradores se van de a uno, primero el que es autor de más líneas, hasta que más del 50% del código queda en archivos sin autor. El bus factor es cuántos se fueron. Los archivos pesan según sus líneas, así que un archivo de una línea no cuenta tanto como el núcleo del proyecto, y una cola larga de colaboradores ocasionales no sube el número.
- Con `--since`, los autores sin commits en esa ventana ya se fueron: el código que solo ellos escribieron queda huérfano antes de contar a nadie. Si eso ya supera el umbral, el bus factor es 0.
- Las personas se distinguen por email, leído a través de `.mailmap`, así que una dirección con varios nombres, o varias direcciones que el mailmap junta, son una sola persona. Esto también vale para `--summary`, `km authors` y `km loc --by-author`.

```
$ km knowledge --bus-factor --since 1y
Project Bus Factor: 2

 12% of the code is already in files whose authors have not committed in 1y.
 If these 2 contributors leave, 71% of the code is in files with no author left.
 Risk: HIGH — two people hold critical knowledge
 Authors of a file hold at least 75% of the lines of its main author; files weigh by lines; threshold 50%.

──────────────────────────────────────────────────────────────────────
 Rank  Author                Files     Lines    Share  Orphaned
──────────────────────────────────────────────────────────────────────
    1  E. Diaz                  41      8420   68.12%     48.3%
    2  A. Torres                 9      1490   12.06%     71.0%
    3  R. Soto                   6       940    7.61%
    4  M. Rojas (inactive)       3       610    4.94%
──────────────────────────────────────────────────────────────────────
```

`Orphaned` es el porcentaje del código que queda sin autor cuando ese colaborador y los que están más arriba se han ido.

### `km tc` -- Análisis de acoplamiento temporal

Analiza el acoplamiento temporal entre archivos a partir del historial de git. Se basa en el método de Adam Thornhill ("Your Code as a Crime Scene", cap. 7): los archivos que cambian juntos a menudo en los mismos commits tienen un acoplamiento implícito, aunque no se importen directamente.

```bash
km tc [path]
```

#### Fórmula

```
Coupling strength = shared_commits / min(commits_a, commits_b)
```

#### Niveles de acoplamiento

| Fuerza | Nivel | Significado |
|----------|-------|---------|
| >= 0.5 | STRONG | Los archivos cambian juntos la mayor parte del tiempo |
| 0.3-0.5 | MODERATE | Patrón apreciable de cambios conjuntos |
| < 0.3 | WEAK | Cambios conjuntos ocasionales |

Un acoplamiento alto entre módulos sin relación sugiere dependencias ocultas o problemas de arquitectura: considera extraer abstracciones compartidas.

Opciones:

| Flag | Descripción |
|------|-------------|
| `--format {table,json,short,terse}` | Formato de salida (por defecto: table) |
| `--top N` | Muestra solo los N primeros pares de archivos (por defecto: 20) |
| `--sort-by METRIC` | Ordena por `strength` o `shared` (por defecto: `strength`) |
| `--since DURATION` | Considera solo los commits desde este momento (p. ej. `6m`, `1y`, `30d`) |
| `--min-degree N` | Mínimo de commits por archivo para incluirlo (por defecto: 3) |
| `--min-strength F` | Fuerza mínima de acoplamiento que se muestra (p. ej. `0.5` para ver solo el fuerte) |

Ejemplo de salida:

```
Temporal Coupling — Files That Change Together
──────────────────────────────────────────────────────────────────────────────────
 File A                     File B                     Shared  Strength  Level
──────────────────────────────────────────────────────────────────────────────────
 src/auth/jwt.rs            src/auth/middleware.rs          12      0.86  STRONG
 lib/parser.rs              lib/validator.rs                 8      0.53  STRONG
 config/db.yaml             config/cache.yaml                6      0.35  MODERATE
──────────────────────────────────────────────────────────────────────────────────

12 coupled pairs found (3 shown). Showing pairs with >= 3 shared commits.
Strong coupling (>= 0.5) suggests hidden dependencies — consider extracting shared abstractions.
```

**Aviso:** los renombres de archivos no se siguen a lo largo del historial de git. Los archivos renombrados aparecen como entradas separadas.

### `km impact` -- Impacto de un diff

Mide hasta dónde llega un cambio, sea un PR o trabajo sin commit, antes de hacer el merge.

#### Qué es el blast radius

El **blast radius** (radio de impacto) de un cambio es la parte del sistema que puede comportarse distinto por su causa, más allá de los archivos que edita. Un cambio en una función no es solo esa función: es todo el código que la llama, y lo que llama a ese código. Si los tests propios de la función pasan y algo que la usa falla en producción, la falla estaba dentro del radio y fuera de los tests.

`km impact` lo mide en dos niveles, e informa qué lo resguarda:

| Nivel | El radio es | Se lee de |
|-------|---------------|-----------|
| Proyectos | Los proyectos del repositorio que dependen de los que cambiaron, directamente o a través de otros | Los manifiestos |
| Archivos fuente | Los archivos que llaman a las funciones que cambiaron, y los archivos que usan a esos | El grafo de dependencias del código |

En cada nivel el radio es una cuenta sobre un total: `3 of 6 projects`, `2 of 618 source files`. Responde **de cuánto del sistema hay que preocuparse**. A su lado viene **lo que está desprotegido**: los archivos dentro del radio que ningún test ejercita. Un radio amplio cubierto por completo con tests es un cambio que hay que hacer con cuidado; un solo archivo del radio sin test es donde se va a romper sin aviso, y el informe lo nombra.

Otras dos medidas describen el cambio en sí y no su alcance: qué tan disperso es (**difusión**), y qué archivos suelen cambiar con él y quedaron fuera (**radio lógico**).


```bash
km impact --since-ref origin/main [path]         # la rama en la que estás
km impact --since-ref main --until-ref feature   # una rama, sin cambiarte a ella
git diff main... | km impact --diff -               # un parche
km impact --pr 123                               # un pull request de GitHub
```

#### Qué se mide

| Origen | El cambio | El historial termina en | Los manifiestos y archivos se leen de |
|--------|------------|-----------------|-------------------------------|
| `--since-ref REF` | Desde donde `REF` y `HEAD` divergieron hasta el árbol de trabajo: cambios con commit, sin commit y sin seguimiento, y eliminaciones | Ese merge base | El árbol de trabajo |
| `--since-ref A --until-ref B` | Lo que `B` trae desde que divergió de `A`, sin importar qué rama tengas activa | Ese merge base | El árbol de `B` |
| `--diff FILE` | Un parche en formato git, desde un archivo o desde stdin (`-`) | `HEAD`, o donde `--since-ref` y `HEAD` divergieron si se indica | El árbol de trabajo |
| `--pr NUMBER` | Un pull request de GitHub | Como en el modo en que se resuelve | Como en el modo en que se resuelve |

Siempre es el cambio sobre todo el repositorio: `path` solo ubica el repositorio y no acota el análisis. Los archivos generados (archivos lock, recursos minificados) quedan fuera de todas las medidas.

**`--pr` requiere la [GitHub CLI](https://cli.github.com) (`gh`) instalada y autenticada.** kimun la ejecuta como un programa: no enlaza ningún cliente de GitHub ni guarda ningún token.

- Cuando el repositorio tiene los commits del pull request, se mide a partir de ellos, como entre dos refs. Un pull request integrado con squash o rebase, cuyo head nunca se trajo con fetch, se mide a partir del parche que sirve GitHub, leído contra el árbol del commit que lo integró. Su base y ese commit no se comparan: la base puede estar muchos merges atrás, y se contaría todo lo que se integró entre medio.
- En otro caso (un pull request desde un fork, o uno que no se trajo con fetch) su parche se obtiene de `gh pr diff` y se mide como cualquier otro parche, con un aviso en stderr. `git fetch origin pull/NUMBER/head` deja disponibles sus commits.

Un parche (`--diff`, o un pull request sin commits locales) se mide contra el árbol de trabajo, donde no está aplicado:

- debe estar en formato git con los prefijos `a/` y `b/`, tal como lo imprimen `git diff`, `git format-patch` y `gh pr diff`, sin colores;
- para una rama usa `git diff main...` (tres puntos): `git diff main` compara contra la punta de `main`, y muestra lo que `main` ganó desde entonces como si la rama lo hubiera deshecho. Para incluir el trabajo sin commit, `--since-ref main` es el camino directo;
- un proyecto que el parche crea no se conoce, así que sus archivos tienen alcance desconocido, y `--affected` lista todos los proyectos;
- si el parche ya está aplicado en `HEAD`, pasa `--since-ref` para que sus propios commits no se cuenten como historial.

#### Blast radius: proyectos

Qué proyectos del repositorio alcanza el diff. Pensado para monorepos, donde un cambio en una biblioteca compartida alcanza aplicaciones que su autor quizás no conoce.

Un **proyecto** es un directorio con un manifiesto. Un proyecto **depende** de otro cuando su manifiesto lo nombra como dependencia local; las dependencias hacia registros u otros repositorios se ignoran. Un archivo modificado pertenece al proyecto más cercano por encima de él. El radio es todo proyecto que depende de uno que cambió, directamente o a través de otros.

| Ecosistema | Manifiesto | Dependencias locales que se leen |
|-----------|----------|-------------------------|
| Rust | `Cargo.toml` | dependencias con `path`, `workspace = true` resuelto a través de `[workspace.dependencies]`, en `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]` y sus formas `[target.*]` |
| JavaScript / TypeScript | `package.json` | cualquier dependencia cuyo nombre sea otro paquete del repositorio (workspaces de npm, yarn y pnpm), además de `file:` y `link:` |
| Elixir | `mix.exs` | dependencias con `path:` e `in_umbrella: true` |
| Go | `go.mod`, `go.work` | módulos requeridos que son otro módulo del repositorio, y `replace` con un directorio |
| Python | `pyproject.toml` | cualquier requisito cuyo nombre sea otro proyecto del repositorio (sin distinguir mayúsculas ni `-`, `_`, `.`), y los que tienen un `path` en `[tool.uv.sources]` o en las tablas de Poetry; se leen `[project]` (`dependencies`, `optional-dependencies`), `[dependency-groups]`, `[build-system] requires`, `[tool.poetry]` y los grupos de desarrollo de uv y PDM |

```
Blast radius — projects reached through their manifests
──────────────────────────────────────────────────────────────────────────────
 3 of 6 projects reached (50%), 2 direct
 Changed: libs/core

 Changed    Reaches         Distance  Scope  Via
 libs/core  apps/inventory         1
 libs/core  libs/locker            1
 libs/core  apps/parcels           2         libs/locker
──────────────────────────────────────────────────────────────────────────────
Changed files outside every project (reach unknown): Makefile
```

- **Scope**: una dependencia `dev` (solo de desarrollo o de test) alcanza al dependiente, cuyos tests usan el proyecto que cambió, y se detiene ahí: el proyecto que cambió no es parte de lo que el dependiente entrega, así que no se alcanza a los dependientes del dependiente. Las dependencias `build` y `optional` siguen adelante igual que las de ejecución.
- **Raíces de workspace**: un `Cargo.toml` con `[workspace]`, un `package.json` con `workspaces` (o junto a un `pnpm-workspace.yaml`), un `mix.exs` umbrella con `apps_path`, un `go.work`, un `pyproject.toml` con `[tool.uv.workspace]`. Un cambio en el manifiesto o en el archivo lock de una de esas raíces (`Cargo.lock`, `package-lock.json`, `yarn.lock`, `pnpm-lock.yaml`, `bun.lock`, `mix.lock`, `uv.lock`) alcanza a todos los proyectos que tiene debajo a distancia 1, con scope `workspace`, y sigue adelante desde ellos. Una raíz de workspace es un proyecto por sí misma solo cuando declara uno (`[package]` en Cargo, `app:` en mix, `[project]` en Python); un `package.json` en una raíz de workspace nunca lo es.
- **Los archivos inertes** no alcanzan nada: un cambio en la documentación (`.md`, `.mdx`, `.rst`, `.adoc`, `.txt`) no rompe ninguna compilación ni ningún test. No cuenta como un cambio en su proyecto, ni como un archivo de alcance desconocido. `.kimun.toml` puede declarar más:

  ```toml
  [impact]
  inert = ["scripts/**", "notebooks/**"]   # globs, relative to the repository
  ```

- **Los archivos fuera de todo proyecto** (workflows de CI, configuración compartida) se listan aparte. Su alcance es desconocido, no cero.

- **`--affected`** imprime los proyectos cuyas compilaciones y tests pide el diff, uno por línea, y nada más: los que cambiaron y los alcanzados. Si algún archivo modificado está fuera de todo proyecto, imprime **todos** los proyectos y explica por qué en stderr: saltarse una suite de tests es peor que ejecutar una de más. Lee solo el diff y los manifiestos, no el historial.
- Un repositorio con un solo proyecto recibe una línea que dice que este nivel no aplica, en vez de "0 reached".
- En `node_modules`, `vendor` y `testdata` nunca se buscan manifiestos, ni en `deps`, `_build` y `target` (salvo que estén bajo `src`, `lib` o `app`, donde son parte del proyecto), ni tampoco en un directorio `fixtures` dentro de un directorio de tests. Una suite de extremo a extremo con su propio manifiesto (`test/e2e/package.json`) es un proyecto.

Límites:

- `mix.exs` es código y se lee como texto. Una dependencia cuya ruta se construye en tiempo de ejecución (`Path.expand(...)`, interpolación de strings, una lista generada) no se puede atribuir; el informe nombra el manifiesto y cuántas se le escaparon.
- De Python solo se lee `pyproject.toml`. Un proyecto que se declara en `setup.py` (que es código) o `setup.cfg` no se ve, ni una dependencia local escrita en `requirements.txt` (`-e ../lib`). Un `pyproject.toml` que solo configura herramientas, sin `[project]` ni `[tool.poetry]`, no es un proyecto. Los extras (`optional-dependencies`) son `optional`; los grupos de dependencias son `dev`.
- El acoplamiento entre ecosistemas no es visible: un cliente web y el servicio cuya API llama no tienen entre ellos ninguna dependencia de manifiesto.
- Un proyecto anidado en otro (`assets/package.json` dentro de una aplicación Phoenix) no tiene dependencia hacia el que lo contiene ni desde él, salvo que un manifiesto la declare.
- El grafo se lee del árbol de trabajo. Los archivos de un proyecto que el diff elimina o mueve ya no pertenecen a ningún proyecto: su alcance es desconocido, y los manifiestos que todavía lo nombran se informan como no leídos.

#### Blast radius: archivos fuente

Dentro de los proyectos que el cambio afecta: qué archivos fuente usan lo que cambió, y cuáles de ellos no ejercita ningún test. Es la pregunta "los tests del módulo que cambié pasan; ¿quién más lo llama?".

La primera línea es la respuesta en corto: cuántos archivos llaman a lo que cambió, y cuántos de ellos no tienen test. `Radius` es la cuenta de los archivos que llaman a lo que cambió, y de los que usan a esos. `Upper bound` es lo que sería sin saber qué funciones cambiaron.

```
Structural radius — source files that use what changed
──────────────────────────────────────────────────────────────────────────────
 1 file calls what changed, 1 of them with no test
 Changed: lib/booking/insights.ex
 Functions: arrange
 Radius: 1 of 6 source files (17%): 1 at distance 1
 Not in the radius: 1 that refer to the module without calling it
 Upper bound, whatever the function: 4 files (67%)

 Tests  Dependent
  none  lib/booking_web/controllers/insight_controller.ex
            calls Insights.arrange
  none  lib/booking/export.ex
            refers to the module without calling it
──────────────────────────────────────────────────────────────────────────────
No test reaches 1 of the files that call what changed; an integration test is probably missing:
  lib/booking_web/controllers/insight_controller.ex
No test in the change exercises a file that uses what changed.
1 more with no test use the module without a call that tells whether the change concerns them.
```

Cómo se mide:

- El grafo de dependencias de `km deps` se lee hacia atrás desde los archivos fuente que cambiaron.
- En Elixir el cambio se **acota a funciones**: las líneas que toca el diff dicen qué funciones cambiaron, y un cambio en una función privada se traslada a las públicas que llegan a ella con llamadas locales. Un archivo que usa el módulo cae entonces en uno de tres casos: **llama** a una función que cambió, **se refiere** al módulo sin llamarlo (un struct, un `import`, un `use`), o solo llama a funciones que el cambio no toca, y no se lista. Cada archivo modificado se acota por separado. Fuera de las funciones, un `alias` o un `require` tocado no cambia ninguna (solo nombra lo que usan las funciones tocadas), y un atributo de módulo tocado cambia las funciones que lo leen. Un `use`, un `import`, un `defstruct` o un atributo que ninguna función lee puede concernir a todas las funciones: ese archivo no se acota, cuenta cada uso de él, y el informe dice qué línea fue. Un archivo nuevo nunca se acota.
- En Python el cambio se acota a los **nombres de nivel superior** del módulo: sus funciones, clases y asignaciones. Un método tocado cambia su clase; un decorador pertenece a lo que decora; un nombre que usa a uno que cambió también cambia, sea privado o no, porque nada impide que otro archivo lo importe. Un archivo que usa el módulo llama a lo que cambió cuando toma uno de esos nombres: `from m import nombre`, `m.nombre` sobre un módulo que importa (también bajo su alias) o, después de `from m import *`, un nombre que luego menciona. Un módulo entregado como valor puede dar cualquiera de sus nombres. Uno importado sin que se lea nada en él solo se refiere al módulo. Los imports, el docstring, un `if` o un `try` que solo elige imports, y el bloque `if __name__ == "__main__":` no cambian ningún nombre. Cualquier otro código al margen izquierdo se ejecuta al importar el módulo y puede concernir a todos los nombres: el archivo no se acota. No se ven: los nombres alcanzados en tiempo de ejecución (`getattr`, un registro llenado por decoradores, `mock.patch("m.nombre")`), ni qué llamadores usan el método que cambió. Un nombre que un paquete reexporta se sigue a través de su `__init__.py`, una distancia más lejos.
- El **radio** parte en los archivos que llaman a lo que cambió y sigue a quienes los usan, archivo por archivo. Un archivo que solo se refiere al módulo, sin una llamada que lo aclare, se lista pero no prolonga el radio: a un schema lo nombra medio proyecto, y seguir todo eso no dice nada. Tampoco se cuentan los archivos que solo pasan por un dependiente que el cambio no toca. Después del primer paso el radio sigue siendo por archivo, no por función, así que es una estimación por arriba. La **cota superior** es lo que sería el radio si contara cada uso de un archivo modificado, sea cual sea la función: en una base de código donde todo pasa por unos pocos contextos es la mayor parte del proyecto, y por eso el radio es el número que hay que leer.
- **Tests** dice cómo está protegido el dependiente. Un test rara vez nombra todo lo que ejercita (un controlador o una live view se prueban a través de su ruta, un helper a través de las vistas que lo usan), así que la protección viene en grados:

  | `Tests` | Protección | Cuándo |
  |---------|------------|------|
  | un número | directa | Esa cantidad de archivos de test se refieren a él, piden una ruta que él sirve, o están en el mismo lugar de la estructura de fuentes y tests (`lib/a/b.ex` y `test/a/b_test.exs`) |
  | `named` | por nombre | Un test lleva su nombre a un directorio de distancia (`live/page_live.ex` y `page_live_test.exs`), o el nombre del directorio en que está (`page_live/index.ex` y `page_live_test.exs`). Un test en el lugar de un archivo fuente es el test de ese archivo y no nombra a ningún otro: `app_test.exs` junto a `app.ex` no dice nada de los archivos de `app/` |
  | `users` | por quienes lo usan | No tiene test propio, pero un archivo que lo usa sí tiene uno |
  | `none` | ninguna | Ningún test llega a él |

  El soporte de tests (`test/support/`), la configuración y los scripts no son tests: una factory se refiere a todo y haría que todo pareciera protegido.
- Un archivo que llama a lo que cambió y al que ningún test llega está **desprotegido**: el cambio puede romperlo sin que ningún test lo note. Esa es la advertencia. Los dependientes se listan desde el menos protegido.
- **Los puntos de entrada** se distinguen del resto. Una tarea de línea de comandos o un script se ejecuta, no se usa: ningún test de otra cosa pasa por él, y pocos tienen uno propio. Sin test reciben una línea propia en lugar de la advertencia. Son los archivos bajo `mix/tasks/`, `management/commands/`, `bin/` y `scripts/` que ningún archivo fuente usa, más lo que declare `.kimun.toml`:

  ```toml
  [impact]
  entry_points = ["**/endpoint.ex"]   # globs, relative to the repository
  ```

- En Elixir, un test que pide una ruta protege al módulo que la sirve. Las rutas se leen del router de Phoenix del proyecto del test, con la ruta y el alias de cada `scope` que las rodea; `:id` coincide con un segmento cualquiera, y un segmento que el test escribe en tiempo de ejecución (`#{order.id}`) coincide solo con un parámetro de la ruta: pedir una orden no protege `/orders/new`.
- En Elixir, lo que un framework relaciona por convención cuenta como un uso. Un controlador de Phoenix usa las vistas que llevan su nombre (`PageController` y `PageJSON`, `PageHTML`, `PageView`), así que el test del controlador las protege. Un módulo usa los componentes que renderizan sus plantillas, ya sea que estén escritas en él con `~H`, guardadas en un archivo `.html.heex` a su lado (`page/index.ex` y `page/index.html.heex`) o en un directorio que lleva su nombre (`page_html.ex` y `page_html/home.html.heex`).


Se mide para los lenguajes cuyo grafo refleja el uso: C, C++, C#, Elixir, Go, Java, JavaScript/TypeScript, Kaikai, Python y Rust. En Go la unidad es el paquete: un cambio en cualquiera de sus archivos alcanza a cada archivo que lo importa. En C y C++ un cambio en un archivo fuente alcanza a los archivos que incluyen su header. Solo se leen los proyectos que el cambio afecta; el repositorio completo cuando el alcance sobre los proyectos es desconocido.

Límites: un test en el mismo lugar, o que lleva el nombre de un archivo, puede no ejercitar la llamada que cambió, y uno que se refiere a un archivo puede simular con mocks lo que este llama: la columna dice que existe un test, no que cubre. `users` es más débil todavía: dice que algo que usa el archivo tiene tests. Los nombres de función se comparan sin aridad. Los módulos nombrados en tiempo de ejecución (`apply/3`, configuración) o generados por macros no se ven. Una petición se lee cuando su ruta está escrita en la llamada (`live(conn, ~p"/orders")`), no cuando viene de una variable o de un helper, y una ruta cuando está declarada en una sola línea.

#### Difusión

Qué tan disperso es el cambio.
 Kamei et al. encontraron que la difusión está entre los predictores más fuertes de un cambio que introduce defectos.

| Medida | Significado |
|---------|---------|
| Archivos modificados | Archivos agregados, modificados, renombrados o eliminados |
| Directorios | Directorios distintos que contienen un archivo modificado |
| Subsistemas | Directorios distintos de primer nivel (los archivos de la raíz forman uno más) |
| Líneas agregadas / eliminadas | Los archivos binarios no cuentan líneas |
| Entropía | Entropía de Shannon de las líneas modificadas sobre los archivos, dividida por su máximo: `0` cuando un solo archivo contiene todas las líneas modificadas, `1` cuando todos los archivos contienen la misma cantidad |

#### Radio lógico

Archivos que suelen cambiar con los archivos del diff y que **no** están en él: un cambio que puede haberse olvidado. Detecta el acoplamiento que el código no declara: tests, configuración, migraciones.

```
Confidence = shared_commits / commits of the changed file
```

Un archivo se informa cuando algún archivo modificado alcanza `--min-confidence` con al menos `--min-shared` commits compartidos. La confianza es direccional, a diferencia de la fuerza de `km tc`: un archivo que cambió tres veces, siempre junto a uno que cambió cien veces, tiene fuerza 1.0, pero hace falta en el 3% de los cambios del otro.

- El historial termina en el merge base: los commits del diff nunca son evidencia de sí mismos.
- Los commits que tocan más de `--max-changeset` archivos se ignoran, y el informe dice cuántos fueron: un reformateo o un renombre en todo el proyecto relaciona sus archivos entre sí por accidente.
- Un archivo modificado sin historial (nuevo, o fuera de `--since`) no predice nada. Se lista aparte, para que su silencio no se lea como "sin impacto".
- Los archivos que ya no existen no se informan.
- Los archivos de test siempre son parte del análisis: un test que suele cambiar con el código es un cambio que conviene esperar.

Opciones:

| Flag | Descripción |
|------|-------------|
| `--since-ref REF` | Ref de git contra la que se compara, p. ej. `origin/main`, `HEAD`. Obligatoria salvo que se indique `--diff` o `--pr` |
| `--until-ref REF` | Mide hasta esta ref en lugar del árbol de trabajo (necesita `--since-ref`) |
| `--diff FILE` | Mide un parche en formato git; `-` lee de stdin |
| `--pr NUMBER` | Mide un pull request de GitHub; requiere `gh` instalado y autenticado |
| `--since DURATION` | Considera solo el historial desde este momento (p. ej. `6m`, `1y`, `30d`) |
| `--min-confidence F` | Confianza mínima para informar un archivo faltante (por defecto: `0.5`) |
| `--min-shared N` | Mínimo de commits compartidos para informar un archivo faltante (por defecto: `3`) |
| `--max-changeset N` | Ignora como evidencia los commits que tocan más de N archivos (por defecto: `30`) |
| `--affected` | Imprime solo los proyectos que cambiaron y los alcanzados, uno por línea |
| `--top N` | Muestra solo los N primeros archivos faltantes (por defecto: 20) |
| `--format {table,json,short,terse}` | Formato de salida (por defecto: table) |

Ejemplo de salida:

```
Change Impact — diff against main

Blast radius — projects reached through their manifests
──────────────────────────────────────────────────────────────────────────────
 Single project (.): no other project to reach; this level does not apply.
──────────────────────────────────────────────────────────────────────────────

Diffusion
  Files changed           5
  Directories             3
  Subsystems              2
  Lines added           120
  Lines deleted          30
  Entropy              0.82  (0 = one file holds the change, 1 = evenly spread)

Logical radius — files that usually change with this diff and are not in it
──────────────────────────────────────────────────────────────────────────────
 Missing file      Confidence   Shared  Changes with
──────────────────────────────────────────────────────────────────────────────
 src/tc/report.rs        0.80     8/10  src/tc/mod.rs (+1 more)
 README.md               0.50     6/12  src/cli.rs
──────────────────────────────────────────────────────────────────────────────
No history before the diff (new or never committed): src/impact/mod.rs
Generated files ignored: 1
```

Cuando las filas son demasiado anchas para una tabla (rutas largas), cada archivo faltante se lista en una línea propia con su evidencia debajo.

#### Salida JSON, para herramientas y LLM

`--format json` lleva todo lo que muestra la tabla, y las listas que la tabla recorta. Un agente que revisa un cambio puede leerlo en este orden:

```bash
km impact --since-ref origin/main --format json
```

| Campo | Significado |
|-------|---------|
| `source` | El cambio medido: `diff against main`, `PR #12`, `patch from stdin` |
| `structural.functions` | Funciones que el cambio afecta (las públicas en Elixir, los nombres de nivel superior en Python), sobre los archivos que permiten saberlo; `null` cuando ninguno se pudo acotar |
| `structural.narrowing[]` | Por archivo modificado: `functions`, o `null` con la razón (`reason`) por la que cuenta cada uso de él |
| `structural.direct[]` | Cada archivo que usa un archivo modificado: `file`, `exposure` (`calls`, `refers`, `elsewhere`), `protection` (`direct`, `named`, `users`, `none`), `calls`, `tests`, `tests_in_diff`, `entry_point` |
| `structural.unprotected[]` | Archivos que llaman a lo que cambió y a los que ningún test llega, ni siquiera a través de lo que los usa: ahí falta un test de integración |
| `structural.untested_entry_points[]` | Tareas de línea de comandos y scripts que llaman a lo que cambió y no tienen test; no se cuentan como desprotegidos |
| `structural.unknown_without_tests[]` | Archivos que usan el módulo modificado sin una llamada que lo aclare, y que no tienen test |
| `structural.change_tests_a_dependent` | Si un test del cambio protege un archivo que usa lo que cambió |
| `structural.radius` | `files`, `source_files` y `share` (de 0 a 1): el radio como número |
| `structural.reach[]` | Los archivos del radio, por `distance` |
| `structural.upper_bound` | Archivos alcanzados si contara cada uso de un archivo modificado, sea cual sea la función |
| `projects.changed[]`, `projects.reached[]` | Proyectos que contienen un archivo modificado, y los alcanzados, cada uno con `origin`, `distance`, `via`, `scope` |
| `projects.affected[]` | Proyectos cuyas compilaciones y tests pide el cambio (lo que imprime `--affected`) |
| `projects.outside[]` | Archivos modificados que no pertenecen a ningún proyecto: su alcance es desconocido |
| `projects.inert[]` | Archivos modificados que no alcanzan nada: documentación, y lo que `.kimun.toml` declara inerte |
| `diffusion` | Archivos, directorios, subsistemas, líneas y entropía del cambio |
| `logical_radius.missing[]` | Archivos que suelen cambiar con el cambio y no están en él, con cada uno de los archivos que los predicen |

`km ai` expone el comando a un LLM como la herramienta `km_impact`, y la skill que instala `km ai skill` lo documenta.

`Shared` se lee como commits compartidos sobre los commits del archivo modificado.
 `(+1 more)` significa que otro archivo modificado predice el mismo archivo faltante; `--format json` los lista todos. `--format terse` imprime el número de archivos faltantes.

**Aviso:** los renombres de archivos no se siguen a lo largo del historial de git. Un archivo renombrado en el propio diff conserva el historial de su ruta anterior.

### `km churn` -- Análisis de churn

Mide la frecuencia de cambio pura por archivo a partir del historial de git (solo la cantidad de commits, sin ponderar por complejidad). Identifica los archivos que más se modifican: un churn alto sin una mejora de calidad que lo acompañe es una señal de mantenimiento.

```bash
km churn [path]
```

Opciones:

| Flag | Descripción |
|------|-------------|
| `--top N` | Muestra solo los N primeros archivos (por defecto: 20) |
| `--sort-by METRIC` | Ordena por `commits` (por defecto), `rate` (commits por mes) o `file` |
| `--since DURATION` | Considera solo los commits desde este momento (p. ej. `6m`, `1y`, `30d`) |
| `--format {table,json,short,terse}` | Formato de salida (por defecto: table) |

Ejemplo de salida:

```
Code Churn — Change Frequency
──────────────────────────────────────────────────────────────────────────────
 File                     Language  Commits   Rate/mo   First Seen   Last Seen
──────────────────────────────────────────────────────────────────────────────
 src/main.rs                  Rust       18      3.2    2025-01-10  2026-03-28
 src/loc/counter.rs           Rust        7      1.3    2025-01-10  2026-02-14
 src/dups/detector.rs         Rust        7      1.2    2025-02-01  2026-02-20
──────────────────────────────────────────────────────────────────────────────
```

### `km smells` -- Detección de code smells

Detecta problemas comunes de calidad del código por archivo con heurísticas basadas en texto (no requiere AST). Solo se analizan los lenguajes que tienen marcadores de complejidad (el mismo conjunto que `km cycom`: Rust, Python, JS/TS, C/C++, Go, etc.).

```bash
km smells [path]
```

#### Tipos de smell

| Smell | Descripción |
|-------|-------------|
| `long_function` | El cuerpo de la función supera `--max-lines` (por defecto: 50) |
| `long_params` | La función tiene más de `--max-params` parámetros (por defecto: 4) |
| `todo_debt` | TODO, FIXME, HACK, XXX o BUG en líneas de comentario |
| `magic_number` | Literales numéricos sueltos en el código (sin contar 0, 1, 2, -1 ni las declaraciones `const`/`let`) |
| `commented_code` | Dos o más líneas de comentario consecutivas con patrones que parecen código |

Opciones:

| Flag | Descripción |
|------|-------------|
| `--top N` | Muestra solo los N primeros archivos por cantidad de smells (por defecto: 20) |
| `--max-lines N` | Máximo de líneas del cuerpo de una función antes de marcarla (por defecto: 50) |
| `--max-params N` | Máximo de parámetros antes de marcarla (por defecto: 4) |
| `--files FILE` | Analiza solo estos archivos (se puede repetir). Útil para scripts |
| `--since-ref REF` | Analiza solo los archivos modificados desde esta ref de git (p. ej. `origin/main`, `HEAD~1`). Ideal para CI |
| `--format {table,json,short,terse,github,codeclimate}` | Formato de salida (por defecto: table). `github` emite anotaciones de GitHub Actions; `codeclimate` (alias: `gitlab`) emite JSON de CodeClimate para GitLab Code Quality |

La tabla desglosa por tipo la cantidad de smells de cada archivo, con una columna por tipo de smell (`magic`, `long`, `param`, `todo`, `comm`) y un total por columna en el pie.

Ejemplo de salida:

```
Code Smells
──────────────────────────────────────────────────────────────────────
 File                            Total  magic  long  param  todo  comm
──────────────────────────────────────────────────────────────────────
 src/loc/counter.rs                 12      7     1      0     4     0
 src/main.rs                         6      2     0      0     4     0
 src/dups/detector.rs                3      0     2      1     0     0
──────────────────────────────────────────────────────────────────────
 Total (3 files)                    21      9     3      1     8     0
```

### `km deps` -- Análisis del grafo de dependencias

Analiza las dependencias internas entre módulos leyendo las sentencias import/use/require. Construye un grafo dirigido del acoplamiento entre archivos y detecta ciclos con el algoritmo SCC de Tarjan.

```bash
km deps [path]
```

Soporta Elixir (cada módulo al que el código se refiere, con el `alias` deshecho; mira las notas más abajo), Rust (cada ruta que nombra un módulo del workspace: `use`, `crate::`, `super::`, un módulo hijo; mira las notas más abajo), Python (`import` y `from … import`, absolutos y relativos; mira las notas más abajo), JavaScript/TypeScript (`import`, `export … from`, `require` e `import()`, relativos o a través de los alias de `tsconfig.json` y de los paquetes del workspace; mira las notas más abajo), Go (imports de los paquetes del repositorio, según la ruta de módulo de cada `go.mod`; mira las notas más abajo), C y C++ (`#include`, como lo encuentra un compilador; mira las notas más abajo), Java (las clases que nombra un archivo, a través de su paquete y de sus imports; mira las notas más abajo), C# (los tipos que nombra un archivo, a través de sus namespaces y de sus directivas `using`; mira las notas más abajo) y Kaikai (`import a.b.c`, incluidas las formas `as` y `.{…}`). Las dependencias externas (crates, paquetes de npm, headers del sistema, la biblioteca estándar de Kaikai) se ignoran.

Los archivos en cualquier otro lenguaje quedan fuera del grafo, en vez de listarse con cero dependencias. El pie de la tabla, el arreglo `unsupported` de la salida JSON y el campo `unsupported:N` del formato short dicen cuántos archivos se omitieron, así que "no medido" nunca se muestra como "sin dependencias".

Notas sobre Elixir:

- Las dependencias son entre módulos, y la mayoría no necesita import, así que cada nombre de módulo en el código cuenta como una referencia: llamadas, structs, `use`, `import`, `require`, `@behaviour`, `defimpl`. Los comentarios, strings, heredocs y sigils se apartan primero, así que un doctest no es una dependencia.
- El `alias` se deshace, incluidas las formas con `as:`, agrupadas y de varias líneas, y `__MODULE__`. La sentencia en sí no es un uso.
- Los `defmodule` anidados reciben el nombre del módulo que los contiene, que se deduce de la indentación tal como la deja `mix format`.
- Un módulo definido en varios archivos (proyectos hechos a partir de una misma plantilla) se resuelve al archivo más cercano al que se refiere a él.
- Los comentarios y los literales no contienen referencias, pero el código que un string interpola sí: `"Total: #{Orders.total(order)}"` usa `Orders`.
- Un controlador de Phoenix usa las vistas que llevan su nombre (`PageController` y `PageJSON`, `PageHTML`, `PageView`), y un módulo usa los componentes que renderizan sus plantillas `~H`.
- No se ven: los módulos nombrados en tiempo de ejecución (`apply/3`, configuración), los generados por macros, y los que un router de Phoenix nombra bajo el alias de un `scope`.

Notas sobre Rust:

- Una dependencia es una ruta: `use crate::git::GitRepo`, una ruta calificada `super::analyzer::run(x)`, una llamada sobre un módulo hijo (`report::print(x)`). Se leen los grupos, `as`, `self`, los globs y los `use` de varias líneas. `mod x;` solo dice dónde vive un módulo y no agrega relación.
- Cada crate es un árbol de módulos que crece desde su raíz (`src/lib.rs`, `src/main.rs`, un archivo de `src/bin`, `tests`, `examples` o `benches`, `build.rs`) por sus declaraciones `mod`, incluido `#[path]`. Una ruta lleva al archivo del módulo más profundo que nombra: el ítem puede estar definido ahí o solo reexportado.
- Los demás targets de un paquete llegan a su librería por el nombre (`my_app::orders` desde `tests/` o `main.rs`), y también los otros crates del workspace. El nombre sale de `[package] name`, con `-` leído como `_`.
- Un tipo usa los archivos que tienen sus bloques `impl`, porque lo que definen se alcanza a través del tipo.
- Para `km impact`, un archivo con funciones `#[test]` tiene un test propio; un archivo de `tests/` que las tiene, o un módulo `tests.rs`, es un test. Lo que un paquete ejecuta (`src/main.rs`, `src/bin`, `examples`, `benches`, `build.rs`) es un punto de entrada.
- Los comentarios, los strings y `$crate` dentro de una macro no nombran nada.
- No se ve: lo que una macro genera o nombra, `include!`, una librería con `[lib] path` o `name` propios, una dependencia renombrada en `Cargo.toml`, y los módulos detrás de `cfg`, que cuentan todos. Un uso hecho solo desde un módulo de tests en el mismo archivo igual convierte al archivo en dependiente. Un test que ejecuta el binario (`assert_cmd`, `CARGO_BIN_EXE_*`) no nombra ningún archivo, así que no protege a ninguno. Dos paquetes con el mismo nombre en un repositorio no se distinguen.

Notas sobre Python:

- Se lee cada `import a.b` y `from a.b import c`, donde sea que esté escrito: en una función, bajo `if TYPE_CHECKING:`, en varias líneas entre paréntesis o con una barra invertida. Uno escrito en un docstring o en un comentario no es un import.
- Una ruta con puntos nombra `a/b.py` o el paquete `a/b/__init__.py`; un stub (`.pyi`) vale por un módulo sin fuente. Solo se usa el módulo más profundo: `import a.b.c` no agrega arista hacia `a/__init__.py`.
- En `from a.b import c`, `c` es el submódulo `a/b/c.py` cuando ese archivo existe, y si no, un nombre que `a.b` define.
- Un import relativo parte del paquete del archivo que importa, un nivel más arriba por cada punto adicional.
- Un import absoluto se busca desde cada directorio sobre el archivo que importa que no sea él mismo un paquete (no tiene `__init__.py`), del más cercano al más lejano, y desde el directorio `src` bajo cada uno. Eso cubre un proyecto que se ejecuta desde su raíz, un layout `src` con `tests/` al lado, y varios proyectos en un repositorio. Después vienen las raíces que declara el `pyproject.toml` más cercano por encima: donde el backend de build encuentra los paquetes (`where` y `package-dir` de setuptools, `packages` de Poetry, `sources` y `packages` de Hatch, `python-source` de maturin, `package-dir` de PDM, `module-root` de uv) y lo que agrega pytest (`pythonpath`). Lo que no se encuentra bajo ninguno es externo: la biblioteca estándar, los paquetes instalados.
- Para `km impact`, `test_*.py`, `*_test.py` y `tests.py` son tests, y `conftest.py` es soporte de tests, que no protege nada; `__main__.py`, `setup.py` y `manage.py` son puntos de entrada, igual que lo que está bajo `management/commands`.
- No se ven: los módulos nombrados en tiempo de ejecución (`importlib`, `__import__`, los settings de Django e `INSTALLED_APPS`), las raíces agregadas en tiempo de ejecución (`sys.path`, `PYTHONPATH`, un archivo `.pth`) o declaradas en `setup.py`, `setup.cfg` o `pytest.ini`, y un paquete namespace repartido en varias raíces. Un nombre que un paquete reexporta lleva a su `__init__.py`, y de ahí al módulo que lo define.

Notas sobre Go:

- Un import nombra un paquete, y un paquete es un directorio: el import depende de cada archivo `.go` que esté directamente dentro, no de los que están en sus subdirectorios, que son otros paquetes. No se lee en qué archivo está lo que se usó.
- Los archivos de test (`_test.go`) no son parte de lo que un paquete ofrece, ni tampoco los que la herramienta ignora (`_x.go`, `.x.go`). Un test en un paquete de test externo (`package store_test`) que importa su propio paquete depende de los archivos que tiene al lado.
- Se lee cada `go.mod` bajo el directorio analizado, así que los módulos de un repositorio con varios se resuelven cada uno desde su propio directorio, con o sin `go.work`. Un import de la ruta del módulo misma es el paquete en la raíz del módulo.
- Los archivos de un mismo paquete se usan entre sí sin importarse, así que no hay aristas entre ellos. El grafo de imports es, por lo tanto, una cota inferior de las dependencias reales.
- No se ven: las restricciones de compilación (`//go:build`, nombres de archivo como `x_linux.go`), así que un archivo que queda fuera de una compilación cuenta igual, ni los paquetes bajo `vendor/`.

Notas sobre JavaScript/TypeScript:

- Solo se leen las formas de import: `import … from`, un `import '…'` solo, `export … from`, `require('…')`, `import('…')` e `import x = require('…')`, varias en una línea o en varias líneas. Cualquier otro string no es un import, así que una ruta que se pasa a `fs.readFile` o a `path.resolve` no es una dependencia.
- Un especificador relativo se resuelve desde el archivo que importa, probando las extensiones de JavaScript y TypeScript y el `index` de un directorio. Un `.js` que no existe nombra su fuente en TypeScript (`./a.js` es `a.ts`), y una query como el `?raw` de Vite nombra el mismo archivo.
- Cualquier otro especificador pasa primero por los `paths` del `tsconfig.json` o `jsconfig.json` más cercano por encima del archivo que importa, y gana el patrón más específico; después, por su `baseUrl`. Se sigue `extends` dentro del repositorio, y un directorio cuyo config principal no tiene alias toma los de los otros que están a su lado (`tsconfig.app.json`). Se aceptan comentarios y comas finales.
- Después, un especificador puede nombrar un paquete del repositorio, por el `name` de su `package.json`: `@acme/ui` lleva a su punto de entrada (`source`, `module`, `main`, `types`, `exports`, leídos desde `src` cuando apuntan a un directorio de build, o si no `src/index`), y `@acme/ui/button` a ese archivo del paquete o de su `src`. Los paquetes bajo `node_modules` no se leen. Lo que no se resuelve por ninguno de estos caminos es externo.
- Un barrel (`index.ts`) es un archivo como cualquier otro: quienes lo importan dependen de él, y él de lo que reexporta.
- No se ve: los alias de la configuración de un bundler (`resolve.alias` de Vite o webpack), los `imports` de `package.json` (`#internal`), los especificadores que se arman en tiempo de ejecución, ni los archivos `.vue` y `.svelte`.

Notas sobre C y C++:

- No se leen los directorios de include del build (`-I`, `include_directories` de CMake). Un `#include "a/b.h"` entre comillas se busca primero junto al archivo que lo incluye; después, con comillas o sin ellas, en cada directorio por encima y en el directorio `include` de cada uno; y por último dondequiera que un archivo del repositorio termine con esa ruta, cuando uno es el más cercano al que incluye.
- `#include <stdio.h>` y cualquier otro nombre suelto entre ángulos que no esté en esos directorios es un header del sistema: no agrega arista, aunque el repositorio tenga en otra parte un archivo con ese nombre.
- Un archivo que incluye un header usa también lo que define su archivo fuente: `foo.h` y el `foo.c` junto a él, o `include/x/foo.h` y `src/x/foo.c`. `km deps` muestra solo los includes que escribe el código, así que un archivo fuente y un header que incluyen la contraparte del otro no forman un ciclo; `km impact` sigue ambos, así que un cambio en `foo.c` alcanza a quien incluye `foo.h`.
- Las condiciones del preprocesador (`#ifdef`) no se evalúan: cuenta cada include.

Notas sobre Java:

- Una clase se conoce por su paquete y su nombre, y vive en el archivo de su clase de nivel superior, dondequiera que esté la raíz de las fuentes (`src/main/java`, `src`, un módulo de un build de varios módulos).
- Un archivo usa las clases que nombra: las de su propio paquete y las de cada import a pedido (`import a.b.*`) por su nombre simple, cada clase que importa por sí sola y cualquier clase escrita con su paquete (`a.b.C`). Una clase anidada (`a.b.Outer.Inner`) y un import estático llevan al archivo de la clase exterior. Un import es un uso aunque nada nombre la clase.
- Cuando varios archivos declaran la misma clase (dos variantes de una biblioteca, una copia generada), se toma la más cercana al archivo que la nombra.
- El JDK y las bibliotecas no declaran ningún archivo del repositorio: sus clases son externas.
- Un campo, una variable o un método con el nombre de una clase no se distingue de ella.

Notas sobre C#:

- Un `using` nombra un namespace, no un archivo. Un tipo se conoce por su namespace (de bloque o de archivo) y su nombre, y un archivo usa los tipos que nombra.
- Un archivo ve por su nombre simple los tipos de cada namespace en que está y de los que lo rodean (`App.Web` ve `App`), del namespace global, de cada `using` y de cada `global using` escrito en su directorio o más arriba. Un alias (`using M = App.Models;`) nombra un tipo o un namespace, `using static` usa el tipo que nombra, y `[Audited]` nombra la clase `AuditedAttribute`.
- Un tipo parcial lo declara cada uno de sus archivos, y usarlo los usa a todos. Cuando dos proyectos declaran un tipo del mismo nombre, se toma el más cercano al archivo que lo nombra.
- Los tipos de .NET y de los paquetes de NuGet no declaran ningún archivo del repositorio: son externos. Los usings implícitos del SDK no traen nada del repositorio.
- Un campo, una variable o un método con el nombre de un tipo no se distingue de él.

Notas sobre Kaikai:

- `import a.b.c` nombra `a/b/c.kai` relativo a la raíz de un paquete, no al archivo que importa. Se prueba cada directorio ancestro del archivo que importa, del más cercano al más lejano, así que ejecuta `km deps` en un directorio que contenga la raíz del paquete.
- Cuando ese archivo no existe, `import a` puede nombrar un directorio de paquete `a/` que tenga un `kai.toml`; el import depende entonces de cada archivo `.kai` que esté directamente dentro.
- Un import que no se resuelve a ningún archivo analizado es externo y no agrega ninguna arista.
- Los archivos de un mismo paquete de Kaikai se fusionan, así que un módulo puede usar un nombre declarado en otro archivo de su paquete sin importarlo. El grafo de imports es, por lo tanto, una cota inferior de las dependencias reales.

| Flag | Descripción |
|------|-------------|
| `--format {table,json,short,terse}` | Formato de salida (por defecto: table) |
| `--cycles-only` | Muestra solo los archivos que participan en un ciclo de dependencias |
| `--sort-by METRIC` | Ordena por `fan-out` (por defecto) o `fan-in` |
| `--top N` | Muestra solo los N primeros archivos (por defecto: 20) |

Ejemplo de salida:

```
Dependency Graph
────────────────────────────────────────────────────────────────────────
 File                 Language Fan-In Fan-Out Cycle
────────────────────────────────────────────────────────────────────────
 report_helpers.rs        Rust     26       1    no
 util.rs                  Rust     25       2   yes
 walk.rs                  Rust     24       1    no
────────────────────────────────────────────────────────────────────────

Dependency cycles: 14
  Cycle 1 (3 files):
    cogcom/analyzer.rs
    cogcom/detection.rs
    cogcom/report.rs
```

### `km authors` -- Resumen de propiedad por autor

Resume la propiedad del código de todo el proyecto por autor. Agrupa los datos de `git blame` para responder "¿quién sabe qué?" a nivel de equipo: complementa a `km knowledge` (vista por archivo) con una vista del equipo.

```bash
km authors [path]
```

Opciones:

| Flag | Descripción |
|------|-------------|
| `--since DURATION` | Considera solo la actividad desde este momento (p. ej. `6m`, `1y`, `30d`) |
| `--format {table,json,short,terse}` | Formato de salida (por defecto: table) |

Ejemplo de salida:

```
──────────────────────────────────────────────────────────────────────
 Author              Owned      Lines  Languages    Last Active
──────────────────────────────────────────────────────────────────────
 E. Diaz                38       8432  Rust, TOML   2026-03-15
 R. Ramirez              4        312  Rust         2026-02-10
──────────────────────────────────────────────────────────────────────
```

### `km age` -- Análisis de antigüedad de archivos

Clasifica los archivos fuente como **Active**, **Stale** o **Frozen** según cuánto tiempo hace que se modificaron por última vez en el historial de git. Ayuda a identificar código descuidado o abandonado.

```bash
km age [path]
```

#### Clasificación por estado

| Estado | Condición | Significado |
|--------|-----------|---------|
| ACTIVE | Modificado dentro de los últimos `--active-days` días (por defecto: 90) | Se toca con regularidad |
| STALE | Entre `--active-days` y `--frozen-days` (por defecto: 365) | Descuidado |
| FROZEN | Sin modificaciones hace más de `--frozen-days` días | Posiblemente abandonado |

Opciones:

| Flag | Descripción |
|------|-------------|
| `--active-days N` | Umbral en días para el estado Active (por defecto: 90) |
| `--frozen-days N` | Umbral en días para el estado Frozen (por defecto: 365) |
| `--sort-by METRIC` | Ordena por `date` (los más antiguos primero, por defecto), `status` o `file` |
| `--status FILTER` | Muestra solo los archivos con este estado: `active`, `stale` o `frozen` |
| `--format {table,json,short,terse}` | Formato de salida (por defecto: table) |

Ejemplo de salida:

```
──────────────────────────────────────────────────────────────────────────────
 File                    Language     Last Modified  Days  Status
──────────────────────────────────────────────────────────────────────────────
 src/legacy/parser.rs    Rust           2023-01-15   840  FROZEN
 src/util.rs             Rust           2024-09-20   197  STALE
 src/main.rs             Rust           2026-03-01    34  ACTIVE
──────────────────────────────────────────────────────────────────────────────

  ACTIVE     12  (modified < 90 days)
  STALE       8  (90 days – 365 days)
  FROZEN      3  (not modified > 365 days)
```

### `km score` -- Puntaje de salud del código

Calcula un puntaje general de salud del código para el proyecto, con una nota que va de A++ (excepcional) a F-- (problemas graves). Usa solo métricas estáticas (no requiere git).

Hay tres modelos de puntaje: `cogcom`, el modelo por defecto, basado en la complejidad cognitiva (5 dimensiones); `cogcom-0.35`, las mismas dimensiones con los pesos que tenían hasta la v0.35; y `legacy`, basado en el índice de mantenibilidad y la complejidad ciclomática (6 dimensiones).

Los archivos que no son código (Markdown, TOML, JSON, etc.) se excluyen automáticamente. Los bloques de test en línea (`#[cfg(test)]`) se excluyen de todas las dimensiones, ver [Tests en línea](#tests-en-línea).

```bash
km score [path]
km score --model cogcom-0.35 [path]  # los pesos hasta la v0.35
km score --model legacy [path]    # modelo de MI + ciclomática
```

#### Dimensiones y pesos (por defecto: cogcom)

| Dimensión | Peso | Qué mide |
|-----------|--------|-----------------|
| Complejidad cognitiva | 32% | Método de SonarSource, penaliza el anidamiento |
| Duplicación | 21% | % de código duplicado en todo el proyecto |
| Complejidad por indentación | 16% | Desviación estándar de la profundidad de indentación |
| Esfuerzo de Halstead | 21% | Esfuerzo mental por LOC |
| Tamaño de archivo | 10% | Puntaje completo hasta 500 LOC; baja a 0 en 4.000 |

Hasta la v0.35 el tamaño de archivo pesaba 15%, y las demás dimensiones 30%, 20%, 15% y 20%. Ahora pesa menos porque el tamaño ya influye en las otras dimensiones: un archivo grande tiene más código que puede ser complejo, y el puntaje del proyecto es un promedio ponderado por líneas de código. Los estudios sobre módulos encuentran que los defectos crecen menos que proporcionalmente con el tamaño, así que partir un archivo no lo hace por sí solo menos propenso a fallas; lo que el tamaño mide bien es el costo de leerlo y cambiarlo. Los cinco puntos pasaron a las demás dimensiones en proporción a sus pesos. `--model cogcom-0.35` calcula con los pesos anteriores, para comparar con resultados previos o para mantener una compuerta de CI donde estaba.

#### Dimensiones y pesos (--model legacy)

| Dimensión | Peso | Qué mide |
|-----------|--------|-----------------|
| Índice de mantenibilidad | 30% | MI de verifysoft, normalizado a 0-100 |
| Complejidad ciclomática | 20% | Complejidad máxima por archivo |
| Duplicación | 15% | % de código duplicado en todo el proyecto |
| Complejidad por indentación | 15% | Desviación estándar de la profundidad de indentación |
| Esfuerzo de Halstead | 15% | Esfuerzo mental por LOC |
| Tamaño de archivo | 5% | Puntaje completo hasta 500 LOC; baja a 0 en 4.000 |

Cada dimensión se agrega como un promedio ponderado por LOC sobre todos los archivos (salvo la duplicación, que es un único valor a nivel de proyecto). El puntaje del proyecto es la suma ponderada de los puntajes de todas las dimensiones.

#### Escala de notas

| Nota | Rango de puntaje | Nota | Rango de puntaje |
|-------|------------|-------|------------|
| A++ | 97-100 | C+ | 73-76 |
| A+ | 93-96 | C | 70-72 |
| A | 90-92 | C- | 67-69 |
| A- | 87-89 | D+ | 63-66 |
| B+ | 83-86 | D | 60-62 |
| B | 80-82 | D- | 57-59 |
| B- | 77-79 | F | 50-56 |
| | | F- | 40-49 |
| | | F-- | 0-39 |

Opciones:

| Flag | Descripción |
|------|-------------|
| `--model MODEL` | Modelo de puntaje: `cogcom` (por defecto), `cogcom-0.35` (los pesos hasta la v0.35) o `legacy` (MI + ciclomática) |
| `--trend [REF]` | Compara el puntaje actual con una ref de git (por defecto: `HEAD`). Muestra el cambio: `B- → B (+2.3)`. Útil para revisar un PR: `--trend origin/main` |
| `--fail-if-worse` | Con `--trend`: termina con código 1 si el puntaje bajó más que `--gate-tolerance` |
| `--gate-tolerance POINTS` | Caída de puntaje que `--fail-if-worse` permite antes de fallar (por defecto: `0.01` con `--gate-scope project`, `0.5` con `--gate-scope changed`). Compara los puntajes sin redondear |
| `--gate-scope {project,changed}` | Qué compara `--fail-if-worse` (por defecto: `project`, el puntaje agregado). `changed` mira solo los archivos que toca el diff: falla si un archivo modificado o renombrado termina por debajo del puntaje que el proyecto tenía en la ref después de bajar más que `--gate-tolerance`, o si crecen las líneas duplicadas del proyecto. Los archivos por encima del puntaje del proyecto, los archivos nuevos y los eliminados nunca la hacen fallar, así que quitar código sano no puede empeorar el veredicto. El informe lista cada archivo modificado con su puntaje de antes y de después |
| `--fail-below GRADE` | Con `--trend`: termina con código 1 si la nota está por debajo de `GRADE` (p. ej. `B-`). Se puede sobrescribir en `.kimun.toml` |
| `--format {table,json,short,terse}` | Formato de salida (por defecto: table) |
| `--include-tests` | Incluye los tests, ejemplos y benchmarks en el análisis (excluidos por defecto) |
| `--bottom N` | Cantidad de peores archivos que se muestran en "needs attention" (por defecto: 10) |
| `--min-lines N` | Mínimo de líneas para un bloque duplicado (por defecto: 6) |

Ejemplo de salida:

```
Code Health Score
──────────────────────────────────────────────────────────────────
 Project Score:  A (91.2)
 Files Analyzed: 142
 Total LOC:      19,084
──────────────────────────────────────────────────────────────────
 Dimension                 Weight   Score   Grade
──────────────────────────────────────────────────────────────────
 Cognitive Complexity         32%    84.1   B+
 Duplication                  21%   100.0   A++
 Indentation Complexity       16%    93.2   A+
 Halstead Effort              21%    87.5   A-
 File Size                    10%   100.0   A++
──────────────────────────────────────────────────────────────────

 Files Needing Attention (worst per-file scores, excl. duplication)
──────────────────────────────────────────────────────────────────
 Score  Grade  File                       Issues
──────────────────────────────────────────────────────────────────
  57.7  D-     ./src/deps/analyzer.rs     Cognitive: 36
  61.0  D      ./src/cogcom/analyzer.rs   Cognitive: 35
  70.3  C      ./src/smells/rules.rs      Cognitive: 20
  71.6  C      ./src/hal/tokenizer.rs     Cognitive: 22
  71.8  C      ./src/loc/mod.rs           Cognitive: 19
  72.8  C      ./src/knowledge/mod.rs     Cognitive: 21
  73.8  C+     ./src/dups/mod.rs          Cognitive: 19
  75.0  C+     ./src/cli_help.rs          Cognitive: 17
  75.6  C+     ./src/test_blocks.rs       Cognitive: 17
  77.3  B-     ./src/walk.rs              Cognitive: 16
──────────────────────────────────────────────────────────────────
```

#### `km score diff` -- Comparar el puntaje con una ref de git

Extrae el árbol de archivos en la ref indicada, calcula el puntaje de ambas instantáneas y muestra una tabla de diferencias por dimensión. Útil para revisar cómo afectan los commits a la calidad del código.

```bash
km score diff                          # compara con HEAD (cambios sin commit)
km score diff --git-ref HEAD~1         # compara con el commit anterior
km score diff --git-ref main           # compara con la rama main
km score diff --format json            # salida legible por máquinas
```

Opciones:

| Flag | Descripción |
|------|-------------|
| `--git-ref REF` | Ref de git con la que se compara (por defecto: `HEAD`) |
| `--model MODEL` | Modelo de puntaje: `cogcom` (por defecto), `cogcom-0.35` o `legacy` |
| `--format {table,json,short,terse}` | Formato de salida (por defecto: table) |
| `--bottom N` | Cantidad de peores archivos que se muestran (por defecto: 10) |
| `--min-lines N` | Mínimo de líneas para un bloque duplicado (por defecto: 6) |

### `km report` -- Informe completo de métricas

Genera un informe de varias secciones que combina todas las métricas estáticas del código en una sola pasada: líneas de código, duplicados, indentación, Halstead, complejidad ciclomática, complejidad cognitiva e índice de mantenibilidad.

```bash
km report [path]
```

Opciones:

| Flag | Descripción |
|------|-------------|
| `--top N` | Muestra solo los N primeros archivos por sección (por defecto: 20) |
| `--min-lines N` | Mínimo de líneas para un bloque duplicado (por defecto: 6) |
| `--full` | Muestra todos los archivos en vez de cortar en los N primeros |
| `--format {table,json,short,terse}` | Formato de salida (por defecto: table) |

## Configuración del proyecto (`.kimun.toml`)

Ejecuta `km init` para analizar tu proyecto y generar un `.kimun.toml` calibrado en un solo paso:

```
$ km init
Analyzing project... done.

Current state:
  avg function length: 38 lines  →  suggested max_lines = 45
  avg param count:     3.2       →  suggested max_params = 4
  dup ratio:           4.1%      →  suggested max_dup_ratio = 5.0
  health score:        B+        →  suggested fail_below = B

Write .kimun.toml with these values? [Y/n]
```

Usa `--yes` para saltarte la pregunta. Agrega `-y` en CI para escribir el archivo sin interacción.

Como alternativa, pon a mano un archivo `.kimun.toml` en la raíz de tu repositorio para fijar los valores por defecto del proyecto para los umbrales y las compuertas de calidad. `km` busca el archivo en la raíz del repositorio git y, si no lo encuentra, en el directorio actual.

Los flags de la línea de comandos siempre tienen precedencia sobre `.kimun.toml`, que a su vez tiene precedencia sobre los valores por defecto incorporados.

```toml
[smells]
max_lines  = 30    # flag functions longer than N body lines (default: 50)
max_params = 3     # flag functions with more than N parameters (default: 4)

[dups]
min_lines      = 8     # minimum block size for duplication detection (default: 6)
                       # also applies to `km report` and `km score`
max_duplicates = 10    # CI gate: fail if duplicate groups exceed N
max_dup_ratio  = 5.0   # CI gate: fail if duplicated-lines ratio exceeds this %

[score]
model      = "cogcom"  # scoring model: cogcom (default), cogcom-0.35 or legacy
fail_below = "B-"      # CI gate: fail if health score is below this grade

[age]
active_days = 60    # files modified within N days are Active (default: 90)
frozen_days = 180   # files not modified for more than N days are Frozen (default: 365)

[tc]
min_degree   = 5    # minimum commits per file to include in coupling analysis (default: 3)
min_strength = 0.5  # only show pairs with coupling strength >= this value

[hotspots]
complexity = "cogcom"  # complexity metric: indent (default), cycom, or cogcom

[impact]
inert = ["scripts/**"]  # changed files that reach nothing, besides documentation
entry_points = ["**/endpoint.ex"]  # files run rather than used, besides tasks and scripts
```

Todas las secciones y todos los campos son opcionales: omite los que no necesites. Hay una plantilla completamente documentada en [`.kimun.toml.example`](.kimun.toml.example).

## Características

- Respeta automáticamente las reglas de `.gitignore`
- Elimina archivos repetidos por hash de contenido (los archivos idénticos se cuentan una vez)
- Detecta los lenguajes por la extensión del archivo, por su nombre o por la línea shebang
- Soporta comentarios de bloque anidados (Rust, Haskell, OCaml, etc.)
- Trata los pragmas (p. ej., `{-# LANGUAGE ... #-}` de Haskell) como código
- Las líneas mixtas (código + comentario) se cuentan como código, igual que en `cloc`
- Lee enteras las fuentes en cualquier codificación de 8 bits (Latin-1, Windows-1252): los bytes que no son UTF-8 se reemplazan, no se toman por el fin del archivo

## Lenguajes soportados

| Lenguaje | Extensiones / Nombres de archivo |
|---|---|
| Bourne Again Shell | `.bash` |
| Bourne Shell | `.sh` |
| C | `.c`, `.h` |
| C# | `.cs` |
| C++ | `.cpp`, `.cxx`, `.cc`, `.hpp`, `.hxx` |
| Clojure | `.clj`, `.cljs`, `.cljc`, `.edn` |
| CSS | `.css` |
| Dart | `.dart` |
| Dockerfile | `Dockerfile` |
| DOS Batch | `.bat`, `.cmd` |
| Elixir | `.ex` |
| Elixir Script | `.exs` |
| Erlang | `.erl`, `.hrl` |
| F# | `.fs`, `.fsi`, `.fsx` |
| Fortran 77 | `.f`, `.for`, `.ftn`, `.f77`, `.pfo` (y en mayúsculas) |
| Fortran 90 | `.f90`, `.F90` |
| Fortran 95 | `.f95`, `.F95` |
| Fortran 2003 | `.f03`, `.F03` |
| Fortran 2008 | `.f08`, `.F08` |
| Go | `.go` |
| Gradle | `.gradle` |
| Groovy | `.groovy` |
| Haskell | `.hs` |
| HTML | `.html`, `.htm` |
| Java | `.java` |
| JavaScript | `.js`, `.mjs`, `.cjs` |
| JSON | `.json` |
| JSX | `.jsx` |
| Julia | `.jl` |
| Kaikai | `.kai` |
| Kotlin | `.kt`, `.kts` |
| Lua | `.lua` |
| Makefile | `.mk`, `Makefile`, `makefile`, `GNUmakefile` |
| Markdown | `.md`, `.markdown` |
| Nim | `.nim` |
| Objective-C | `.m`, `.mm` |
| OCaml | `.ml`, `.mli` |
| Pascal | `.pas`, `.dpr`, `.dpk`, `.lpr`, `.pp`, `.pascal` |
| Perl | `.pl`, `.pm` |
| PHP | `.php` |
| Properties | `.properties` |
| Python | `.py`, `.pyi` |
| R | `.r`, `.R` |
| Ruby | `.rb`, `Rakefile`, `Gemfile` |
| Rust | `.rs` |
| Scala | `.scala`, `.sc`, `.sbt` |
| SQL | `.sql` |
| Swift | `.swift` |
| Terraform | `.tf` |
| Text | `.txt` |
| TOML | `.toml` |
| TypeScript | `.ts`, `.tsx`, `.mts`, `.cts` |
| XML | `.xml`, `.xsl`, `.xslt`, `.svg`, `.fsproj`, `.csproj`, `.vbproj`, `.vcxproj`, `.sln`, `.plist`, `.xaml` |
| YAML | `.yaml`, `.yml` |
| Zig | `.zig` |
| Zsh | `.zsh` |

### Notas específicas por lenguaje

- **Kaikai**: `#[...]` abre un atributo, no un comentario `#`, así que los atributos
  cuentan como código. Los atributos de documentación (`#[doc("...")]`, incluida la
  forma de varias líneas `#[doc("""...""")]`) cuentan como comentarios y se excluyen
  de los análisis de complejidad, de Halstead y de smells.
- **Elixir**: los heredocs de `@moduledoc` y `@doc` (`"""`, `'''` y los sigilos
  `~S"""` y `~s"""`) cuentan como comentarios, como los cuenta cloc, y se excluyen
  de los análisis de complejidad, de Halstead y de smells. Un `@doc "..."` de una
  línea y cualquier otro heredoc cuentan como código.
- **Fortran**: la forma libre (`.f90` y posteriores) comenta con `!`; la forma
  fija (`.f`, `.for`, `.f77`) además hace comentario de una línea con `C`, `c` o
  `*` en la primera columna. Las directivas de OpenMP y OpenACC (`!$omp`,
  `C$OMP`, `!$acc`) y la compilación condicional (`!$ `) cuentan como código,
  porque son directivas; cloc las cuenta como comentarios, y en forma libre
  también toma por comentario una línea que empieza con `c` en la primera
  columna (`contains`, `call`). En un string la barra invertida no escapa nada.
  Fortran no distingue mayúsculas de minúsculas, y tampoco lo hacen los análisis
  de complejidad, de Halstead y de smells, aunque los reportes nombran las
  funciones como están escritas. Las funciones son las unidades de programa
  (`subroutine`, `function`, `program`) hasta el `end` que las cierra, con las
  unidades que contienen; las de un bloque `interface` no declaran funciones. La
  anidación cognitiva se lee de las construcciones (`if … then`, `do`, `select`,
  `where`) y del `end` de cada una, no de la sangría, que la forma fija no tiene.
- **Pascal (Delphi, Free Pascal)**: tres clases de comentario: `//`, `{ … }` y
  `(* … *)`; los dos últimos se cierran solo con su propio delimitador. `{$…}` es
  una directiva del compilador y cuenta como código. Los strings van entre
  comillas simples, sin escapes con barra invertida; un delimitador dentro de uno
  no abre comentario (cloc no los distingue, así que suele contar código como
  comentario después de un `'{'` o de un `COUNT(*)`). Pascal no distingue
  mayúsculas de minúsculas, igual que Fortran. Las funciones son las rutinas de la
  implementación (`procedure`, `function`, `constructor`, `destructor`,
  `operator`, métodos de clase), desde su encabezado hasta el `end` de su bloque,
  con sus rutinas locales; las declaraciones de la sección `interface` y de los
  tipos de clase, y las `forward` o `external`, no tienen cuerpo. Los encabezados
  repetidos bajo compilación condicional son una sola rutina. La anidación
  cognitiva se lee de la sangría, dos columnas por nivel.
- **Las líneas interiores de los strings de varias líneas** (los literales con
  triple comilla de Kaikai y Python, los heredocs de Elixir) cuentan como código
  para `km loc`, pero se excluyen de los análisis ciclomático, cognitivo, de
  Halstead y de smells: la prosa y los datos incrustados no son flujo de control.
- **Los strings crudos de Rust** (`r"..."`, `r#"..."#` y las formas con más `#`,
  también con prefijo `b` o `c`) terminan solo en una comilla seguida de tantos `#`
  como los que los abrieron, y abarcan varias líneas. Lo que contienen cuenta como
  código para `km loc`, se parezca a lo que se parezca, y queda fuera de los
  análisis ciclomático, cognitivo, de Halstead y de smells.

## Desarrollo

```bash
cargo build              # compila el binario de depuración
cargo test               # ejecuta todos los tests
cargo clippy             # lint (se exigen cero advertencias)
cargo tarpaulin --out stdout  # informe de cobertura
```

## Referencias

Las métricas y metodologías implementadas en Kimün se basan en las siguientes fuentes:

### Libros

- **Adam Thornhill**, *Your Code as a Crime Scene* (Pragmatic Bookshelf, 2015). Base del análisis de hotspots (caps. 4–5), del acoplamiento temporal (cap. 7), de los mapas de conocimiento y la propiedad del código (caps. 8–9), y de la complejidad por indentación como indicador indirecto de la calidad del código.
- **Adam Thornhill**, *Software Design X-Rays* (Pragmatic Bookshelf, 2018). Extiende la metáfora de la escena del crimen con más técnicas de análisis del comportamiento del código.

### Artículos y estándares

- **Maurice H. Halstead**, *Elements of Software Science* (Elsevier, 1977). Define las métricas de operadores y operandos: vocabulario, volumen, dificultad, esfuerzo, bugs estimados y tiempo de desarrollo.
- **Thomas J. McCabe**, "A Complexity Measure", *IEEE Transactions on Software Engineering*, SE-2(4), diciembre de 1976, pp. 308–320. Presenta la complejidad ciclomática como medida de los caminos independientes en el grafo de flujo de control de un programa.
- **Paul Oman & Jack Hagemeister**, "Metrics for Assessing a Software System's Maintainability", *Proceedings of the International Conference on Software Maintenance (ICSM)*, 1992. Fórmula original del índice de mantenibilidad, que combina el volumen de Halstead, la complejidad ciclomática y las líneas de código.
- **Microsoft**, [Code Metrics — Maintainability Index range and meaning](https://learn.microsoft.com/en-us/visualstudio/code-quality/code-metrics-maintainability-index-range-and-meaning). Variante de Visual Studio: normalizada a una escala de 0 a 100, sin el término de peso de los comentarios.
- **Verifysoft**, [Maintainability Index](https://www.verifysoft.com/en_maintainability.html). Fórmula extendida del MI con un componente de peso de los comentarios (MIcw) que premia el código bien comentado.
- **Yasutaka Kamei et al.**, "A Large-Scale Empirical Study of Just-in-Time Quality Assurance" (IEEE TSE 39(6), 2013). Base de las medidas de difusión de `km impact`.
- **Thomas Zimmermann, Andreas Zeller, Peter Weissgerber, Stephan Diehl**, "Mining Version Histories to Guide Software Changes" (IEEE TSE 31(6), 2005). Base del radio lógico de `km impact`.

## El nombre

<p align="center">
  <img src="assets/madu.png" alt="Madu, la mascota de Kimün" width="220">
</p>

*Kimün* significa "conocimiento" o "sabiduría" en mapudungun, la lengua del pueblo mapuche.

La mascota es **Madu**, una pudú que es *machi*: la que guarda el conocimiento de su pueblo. Es hermana de Kalku, la mascota de [kalku](https://github.com/lnds/kalku). Ella explica Kimün en [un video de dos minutos](https://kimun.tools/#video):

<p align="center">
  <a href="https://kimun.tools/#video"><img src="assets/video-es.jpg" alt="Reproducir el video: Madu, la mascota, junto a las palabras 'kimün = conocimiento'" width="720"></a>
</p>

## Licencia

MIT. Mira [LICENSE](LICENSE).
