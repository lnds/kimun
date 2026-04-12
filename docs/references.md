# References

The metrics and methodologies implemented in Kimün are based on the following sources.

## Books

- **Adam Thornhill**, *Your Code as a Crime Scene* (Pragmatic Bookshelf, 2015). Basis for hotspot analysis (ch. 4–5), temporal coupling (ch. 7), knowledge maps / code ownership (ch. 8–9), and indentation-based complexity as a proxy for code quality.
- **Adam Thornhill**, *Software Design X-Rays* (Pragmatic Bookshelf, 2018). Extends the crime scene metaphor with additional behavioral code analysis techniques.

## Papers and standards

- **Maurice H. Halstead**, *Elements of Software Science* (Elsevier, 1977). Defines the operator/operand metrics: vocabulary, volume, difficulty, effort, estimated bugs, and development time.
- **Thomas J. McCabe**, "A Complexity Measure", *IEEE Transactions on Software Engineering*, SE-2(4), December 1976, pp. 308–320. Introduces cyclomatic complexity as a measure of independent paths through a program's control flow graph.
- **Paul Oman & Jack Hagemeister**, "Metrics for Assessing a Software System's Maintainability", *Proceedings of the International Conference on Software Maintenance (ICSM)*, 1992. Original Maintainability Index formula combining Halstead Volume, cyclomatic complexity, and lines of code.
- **Microsoft**, [Code Metrics — Maintainability Index range and meaning](https://learn.microsoft.com/en-us/visualstudio/code-quality/code-metrics-maintainability-index-range-and-meaning). Visual Studio variant: normalized to 0–100 scale, no comment-weight term.
- **Verifysoft**, [Maintainability Index](https://www.verifysoft.com/en_maintainability.html). Extended MI formula with a comment-weight component (MIcw) that rewards well-commented code.
