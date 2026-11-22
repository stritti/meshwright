# Roadmap

1. **Skeleton** (done): plugin pipeline, document writer, mesh fallback, quality gates,
   releases.
2. **Plates** (done): any outline of straight sides with round through holes and rounded
   vertical corners.
3. **Parity with mesh2cad**, one recogniser per pull request, each compared with mesh2cad's
   output (`scripts/compare-mesh2cad.sh`, see [Development](/development)):
   1. (done) parameter names as mesh2cad writes them (`base_t`, `fillet_r`, `extrude_len`, …),
      and the part kept where it is in the file
   2. (done) hole rows, grids and bolt circles as SolidCraft patterns
   3. (done) counterbored and countersunk holes
   4. (done) blind holes
   5. (done) slots
   6. (done) bosses on the top face
   7. (done) polygonal cut-outs
   8. steps, pockets and shallow recesses
   9. extrusions along X and Y
   10. boxes with every edge rounded, holes and bosses on side faces
   11. chamfers, drafts and the rounds at a boss's foot or a hole's rim
   12. combined extrusions
4. **Checking**: compare the rebuilt part with the mesh through SolidCraft's `evaluate` (volume
   and deviation), as mesh2cad does with CadQuery.

Contributions are very welcome: bug reports, STL files that import badly, recognisers, docs.
Start with [CONTRIBUTING.md](https://github.com/stritti/meshwright/blob/main/CONTRIBUTING.md).
