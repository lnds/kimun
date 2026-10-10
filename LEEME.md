# Kimün (km)

<p align="center">
  <img src="assets/madu.png" alt="Madu, la mascota de Kimün" width="220">
</p>

> *Kimün* significa "conocimiento" o "sabiduría" en mapudungun, la lengua del pueblo mapuche.
>
> La mascota es **Madu**, una pudú que es *machi*: la que guarda el conocimiento de su pueblo. Es hermana de Kalku, la mascota de [kalku](https://github.com/lnds/kalku).

**Sitio web: [kimun.tools](https://kimun.tools/)** · También [en inglés](README.md).

<p align="center">
  <a href="https://kimun.tools/#video"><img src="assets/video-es.jpg" alt="Reproducir el video: Madu, la mascota, junto a las palabras 'kimün = conocimiento'" width="720"></a>
</p>

**¿No conoces Kimün? [Mira a Madu explicarlo en dos minutos](https://kimun.tools/#video)**: la nota, las métricas, lo que git recuerda, el impacto de un cambio y la compuerta para CI.

Una herramienta de línea de comandos rápida para analizar código, escrita en Rust. Ejecuta `km score` en cualquier proyecto para obtener una nota general de salud (de A++ a F--) en cinco dimensiones de calidad (complejidad cognitiva, duplicación, profundidad de indentación, esfuerzo de Halstead y tamaño de archivo), junto con la lista de los archivos que más atención necesitan.

> Aviso: este repositorio no es la aplicación de notas para la consola `kimun`, escrita en Rust por nico2sh. Ese proyecto está en https://github.com/nico2sh/kimun.

Además del puntaje agregado, Kimün ofrece 17 comandos especializados:

- **Métricas estáticas**: líneas de código por lenguaje (compatible con [cloc](https://github.com/AlDanial/cloc)), detección de código duplicado (Regla de Tres), complejidad de Halstead, complejidad ciclomática, complejidad cognitiva (SonarSource), complejidad por indentación, dos variantes del índice de mantenibilidad (Visual Studio y verifysoft), detección de code smells y un informe completo con todas las métricas.
- **Análisis basado en git**: detección de hotspots (frecuencia de cambio × complejidad, método de Thornhill), churn (frecuencia de cambio pura), propiedad del código y mapas de conocimiento con `git blame`, acoplamiento temporal entre archivos que cambian juntos, impacto de un diff (difusión y cambios conjuntos que faltan), resumen de propiedad por autor y clasificación de archivos por antigüedad (Active / Stale / Frozen).
- **Análisis con IA**: integración opcional con Claude para ejecutar todas las herramientas y producir un informe narrativo.

## Instalación

```bash
cargo install kimun              # desde crates.io
brew install lnds/kimun/kimun    # con Homebrew, en macOS y Linux
```

Cualquiera de los dos instala el binario `km`. También hay binarios listos para macOS, Linux y Windows en la [página de releases](https://github.com/lnds/kimun/releases). Desde una copia de este repositorio, `cargo install --path .` lo compila desde el código fuente.

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

Opciones:

| Flag | Descripción |
|------|-------------|
| `-v`, `--verbose` | Muestra estadísticas de resumen (archivos leídos, únicos, ignorados, tiempo transcurrido) |
| `--by-author` | Desglosa las líneas de código por autor de git (requiere un repositorio git) |
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
| `--include-tests` | Incluye los archivos de test en el análisis (excluidos por defecto) |
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

- **Directorios**: `tests/`, `test/`, `__tests__/`, `spec/`
- **Por extensión**: `*_test.rs`, `*_test.go`, `test_*.py`, `*.test.js`, `*.spec.ts`, `*Test.java`, `*_test.cpp` y más

Usa `--include-tests` para analizar también los archivos de test.

### `km indent` -- Complejidad por indentación

Mide la complejidad de cada archivo a partir de su indentación: la desviación estándar de las profundidades de indentación y la profundidad máxima. Una desviación estándar más alta sugiere un flujo de control más complejo.

```bash
km indent [path]
```

Opciones:

| Flag | Descripción |
|------|-------------|
| `--format {table,json,short,terse}` | Formato de salida (por defecto: table) |
| `--include-tests` | Incluye los archivos de test en el análisis (excluidos por defecto) |

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
| `--include-tests` | Incluye los archivos de test en el análisis (excluidos por defecto) |
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

Rust, Python, JavaScript, TypeScript, Go, C, C++, C#, Java, Objective-C, PHP, Dart, Ruby, Kotlin, Swift, Shell (Bash/Zsh).

### `km cycom` -- Complejidad ciclomática

Calcula la complejidad ciclomática por archivo y por función contando los puntos de decisión (`if`, `for`, `while`, `match`, `&&`, `||`, etc.).

```bash
km cycom [path]
```

Opciones:

| Flag | Descripción |
|------|-------------|
| `--format {table,json,short,terse,github,codeclimate}` | Formato de salida (por defecto: table). `github` emite anotaciones de GitHub Actions; `codeclimate` (alias: `gitlab`) emite JSON de CodeClimate para GitLab Code Quality |
| `--include-tests` | Incluye los archivos de test en el análisis (excluidos por defecto) |
| `--top N` | Muestra solo los N primeros archivos (por defecto: 20) |
| `--min-complexity N` | Omite los archivos cuya función más compleja está por debajo de N; con `--per-function`, oculta también las funciones por debajo de N (por defecto: 1) |
| `--per-function` | Muestra el desglose por función |

### `km cogcom` -- Complejidad cognitiva

Calcula la complejidad cognitiva por archivo y por función con el [método de SonarSource](https://www.sonarsource.com/docs/CognitiveComplexity.pdf) (2017). A diferencia de la complejidad ciclomática, la complejidad cognitiva mide qué tan difícil es *entender* el código: penaliza las estructuras muy anidadas y premia el flujo de control lineal.

```bash
km cogcom [path]
```

Opciones:

| Flag | Descripción |
|------|-------------|
| `--format {table,json,short,terse,github,codeclimate}` | Formato de salida (por defecto: table). `github` emite anotaciones de GitHub Actions; `codeclimate` (alias: `gitlab`) emite JSON de CodeClimate para GitLab Code Quality |
| `--include-tests` | Incluye los archivos de test en el análisis (excluidos por defecto) |
| `--top N` | Muestra solo los N primeros archivos (por defecto: 20) |
| `--min-complexity N` | Omite los archivos cuya función más compleja está por debajo de N; con `--per-function`, oculta también las funciones por debajo de N (por defecto: 1) |
| `--per-function` | Muestra el desglose por función |
| `--sort-by METRIC` | Ordena por `total`, `max` o `avg` (por defecto: `total`) |

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
| `--include-tests` | Incluye los archivos de test en el análisis (excluidos por defecto) |
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
| `--include-tests` | Incluye los archivos de test en el análisis (excluidos por defecto) |
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
| `--include-tests` | Incluye los archivos de test en el análisis (excluidos por defecto) |
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

Identifica el riesgo de bus factor y la concentración del conocimiento por archivo. Los archivos generados (archivos lock, JS minificado, etc.) se excluyen automáticamente.

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
| `--include-tests` | Incluye los archivos de test en el análisis (excluidos por defecto) |
| `--top N` | Muestra solo los N primeros archivos (por defecto: 20) |
| `--sort-by METRIC` | Ordena por `concentration`, `diffusion` o `risk` (por defecto: `concentration`) |
| `--since DURATION` | Define la ventana de actividad reciente para la pérdida de conocimiento (p. ej. `6m`, `1y`, `30d`) |
| `--risk-only` | Muestra solo los archivos con riesgo de pérdida de conocimiento |
| `--summary` | Agrupa por autor: archivos de los que es dueño, líneas, lenguajes, peor riesgo |
| `--bus-factor` | Muestra el bus factor del proyecto (el mínimo de colaboradores que cubren el 80% del código) |
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

Usa `--bus-factor` para calcular cuántos colaboradores te puedes permitir perder:

```
$ km knowledge --bus-factor
Project Bus Factor: 2

 Losing 2 key contributors would put 80% of the project's knowledge at risk.
 Risk: HIGH — two people hold critical knowledge

──────────────────────────────────────────────
 Rank  Author        Lines    Share  Cumulative
──────────────────────────────────────────────
    1  E. Diaz        8420   68.12%     68.12%
    2  A. Torres      1490   12.06%     80.18%  ← 80% threshold
    3  R. Soto         940    7.61%     87.79%
──────────────────────────────────────────────
```

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
| Python | `pyproject.toml` | cualquier requisito cuyo nombre sea otro proyecto del repositorio (sin distinguir mayúsculas ni `-`, `_`, `.`), y los que tienen un `path` en `[tool.uv.sources]` o en las tablas de Poetry; se leen `[project]` (`dependencies`, `optional-dependencies`), `[dependency-groups]`, `[build-system] requires` y `[tool.poetry]` |

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


Se mide para los lenguajes cuyo grafo refleja el uso: Elixir, JavaScript/TypeScript, Kaikai, Python y Rust. Para Go el bloque dice que no está disponible, en vez de informar un radio trazado sobre declaraciones. Solo se leen los proyectos que el cambio afecta; el repositorio completo cuando el alcance sobre los proyectos es desconocido.

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
| `structural.unavailable[]` | Lenguajes de archivos modificados para los que no se mide el nivel de archivos fuente |
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

Soporta Elixir (cada módulo al que el código se refiere, con el `alias` deshecho; mira las notas más abajo), Rust (cada ruta que nombra un módulo del workspace: `use`, `crate::`, `super::`, un módulo hijo; mira las notas más abajo), Python (`import` y `from … import`, absolutos y relativos; mira las notas más abajo), JavaScript/TypeScript (`import`/`require` relativos), Go (imports que coinciden con la ruta del módulo en `go.mod`) y Kaikai (`import a.b.c`, incluidas las formas `as` y `.{…}`). Las dependencias externas (crates, paquetes de npm, la biblioteca estándar de Kaikai) se ignoran.

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

> **Cambio incompatible en v0.14:** el modelo de puntaje por defecto pasó de MI + complejidad ciclomática (6 dimensiones) a complejidad cognitiva (5 dimensiones). Usa `--model legacy` para recuperar el comportamiento de v0.13.

Los archivos que no son código (Markdown, TOML, JSON, etc.) se excluyen automáticamente. Los bloques de test en línea (`#[cfg(test)]`) se excluyen del análisis de duplicación.

```bash
km score [path]
km score --model legacy [path]    # modelo de puntaje de v0.13
```

#### Dimensiones y pesos (por defecto: cogcom)

| Dimensión | Peso | Qué mide |
|-----------|--------|-----------------|
| Complejidad cognitiva | 30% | Método de SonarSource, penaliza el anidamiento |
| Duplicación | 20% | % de código duplicado en todo el proyecto |
| Complejidad por indentación | 15% | Desviación estándar de la profundidad de indentación |
| Esfuerzo de Halstead | 20% | Esfuerzo mental por LOC |
| Tamaño de archivo | 15% | Rango óptimo de 50 a 300 LOC |

#### Dimensiones y pesos (--model legacy)

| Dimensión | Peso | Qué mide |
|-----------|--------|-----------------|
| Índice de mantenibilidad | 30% | MI de verifysoft, normalizado a 0-100 |
| Complejidad ciclomática | 20% | Complejidad máxima por archivo |
| Duplicación | 15% | % de código duplicado en todo el proyecto |
| Complejidad por indentación | 15% | Desviación estándar de la profundidad de indentación |
| Esfuerzo de Halstead | 15% | Esfuerzo mental por LOC |
| Tamaño de archivo | 5% | Rango óptimo de 50 a 300 LOC |

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
| `--model MODEL` | Modelo de puntaje: `cogcom` (por defecto, v0.14+) o `legacy` (MI + ciclomática, v0.13) |
| `--trend [REF]` | Compara el puntaje actual con una ref de git (por defecto: `HEAD`). Muestra el cambio: `B- → B (+2.3)`. Útil para revisar un PR: `--trend origin/main` |
| `--fail-if-worse` | Con `--trend`: termina con código 1 si el puntaje bajó más que `--gate-tolerance` |
| `--gate-tolerance POINTS` | Caída de puntaje que `--fail-if-worse` permite antes de fallar (por defecto: `0.01` con `--gate-scope project`, `0.5` con `--gate-scope changed`). Compara los puntajes sin redondear |
| `--gate-scope {project,changed}` | Qué compara `--fail-if-worse` (por defecto: `project`, el puntaje agregado). `changed` mira solo los archivos que toca el diff: falla si un archivo modificado o renombrado termina por debajo del puntaje que el proyecto tenía en la ref después de bajar más que `--gate-tolerance`, o si crecen las líneas duplicadas del proyecto. Los archivos por encima del puntaje del proyecto, los archivos nuevos y los eliminados nunca la hacen fallar, así que quitar código sano no puede empeorar el veredicto. El informe lista cada archivo modificado con su puntaje de antes y de después |
| `--fail-below GRADE` | Con `--trend`: termina con código 1 si la nota está por debajo de `GRADE` (p. ej. `B-`). Se puede sobrescribir en `.kimun.toml` |
| `--format {table,json,short,terse}` | Formato de salida (por defecto: table) |
| `--include-tests` | Incluye los archivos de test en el análisis (excluidos por defecto) |
| `--bottom N` | Cantidad de peores archivos que se muestran en "needs attention" (por defecto: 10) |
| `--min-lines N` | Mínimo de líneas para un bloque duplicado (por defecto: 6) |

Ejemplo de salida:

```
Code Health Score
──────────────────────────────────────────────────────────────────
 Project Score:  B+ (84.3)
 Files Analyzed: 42
 Total LOC:      8,432
──────────────────────────────────────────────────────────────────
 Dimension                 Weight   Score   Grade
──────────────────────────────────────────────────────────────────
 Cognitive Complexity         30%    85.6   B+
 Duplication                  20%    91.3   A
 Indentation Complexity       15%    79.8   B-
 Halstead Effort              20%    85.1   B+
 File Size                    15%    89.2   A-
──────────────────────────────────────────────────────────────────

 Files Needing Attention (worst scores)
──────────────────────────────────────────────────────────────────
 Score  Grade  File                       Issues
──────────────────────────────────────────────────────────────────
  54.2  F      src/legacy/parser.rs       Cognitive: 42, Indent: 3.2
  63.7  D+     src/utils/helpers.rs       Effort: 15200, Indent: 2.4
  68.9  C-     src/core/engine.rs         Size: 1243 LOC
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
| `--model MODEL` | Modelo de puntaje: `cogcom` (por defecto) o `legacy` |
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
model      = "cogcom"  # scoring model: cogcom (default) or legacy
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
| Go | `.go` |
| Gradle | `.gradle` |
| Groovy | `.groovy` |
| Haskell | `.hs` |
| HTML | `.html`, `.htm` |
| Java | `.java` |
| JavaScript | `.js`, `.mjs`, `.cjs` |
| JSON | `.json` |
| Julia | `.jl` |
| Kaikai | `.kai` |
| Kotlin | `.kt`, `.kts` |
| Lua | `.lua` |
| Makefile | `.mk`, `Makefile`, `makefile`, `GNUmakefile` |
| Markdown | `.md`, `.markdown` |
| Nim | `.nim` |
| Objective-C | `.m`, `.mm` |
| OCaml | `.ml`, `.mli` |
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
| TypeScript | `.ts`, `.mts`, `.cts` |
| XML | `.xml`, `.xsl`, `.xslt`, `.svg`, `.fsproj`, `.csproj`, `.vbproj`, `.vcxproj`, `.sln`, `.plist`, `.xaml` |
| YAML | `.yaml`, `.yml` |
| Zig | `.zig` |
| Zsh | `.zsh` |

### Notas específicas por lenguaje

- **Kaikai**: `#[...]` abre un atributo, no un comentario `#`, así que los atributos
  cuentan como código. Los atributos de documentación (`#[doc("...")]`, incluida la
  forma de varias líneas `#[doc("""...""")]`) cuentan como comentarios y se excluyen
  de los análisis de complejidad, de Halstead y de smells.
- **Las líneas interiores de los strings de varias líneas** (los literales con
  triple comilla de Kaikai y Python) cuentan como código para `km loc`, pero se
  excluyen de los análisis ciclomático, cognitivo, de Halstead y de smells: la
  prosa y los datos incrustados no son flujo de control.

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

## Licencia

Mira [Cargo.toml](Cargo.toml) para los detalles del paquete.
