# MB16-43 native / Rust parity

Result: **PASS**. 129 of 129 pairs compared, 0 failed.

tblite 0.8.0, Windows-11-10.0.26300-SP0, 2026-11-09T13:57:28.168499+00:00. SAD guess, accuracy 0.01, at most 250 SCF iterations, kT = 0.000950042573563535 Eh (300 K), one OpenMP/BLAS thread.

| Method | Property | Max absolute error | RMSE | Bound | Unit | Worst molecule | Pass |
| --- | --- | ---: | ---: | ---: | --- | --- | --- |
| gfn1 | energy | 2.132e-14 | 9.797e-15 | 1.0e-08 | Eh | MB16-43-13 | True |
| gfn1 | energies | 4.796e-14 | 3.703e-15 | 1.0e-08 | Eh | MB16-43-42 | True |
| gfn1 | gradient | 1.359e-14 | 6.103e-16 | 1.0e-07 | Eh/Bohr | MB16-43-13 | True |
| gfn1 | virial | 3.272e-14 | 3.058e-15 | 1.0e-06 | Eh | MB16-43-13 | True |
| gfn1 | charges | 1.241e-13 | 8.880e-15 | 1.0e-06 | e | MB16-43-42 | True |
| gfn1 | dipole | 5.873e-13 | 9.421e-14 | 1.0e-05 | e Bohr | MB16-43-13 | True |
| gfn1 | quadrupole | 2.476e-12 | 4.233e-13 | 1.0e-04 | e Bohr^2 | MB16-43-13 | True |
| gfn2 | energy | 2.487e-14 | 8.109e-15 | 1.0e-08 | Eh | MB16-43-32 | True |
| gfn2 | energies | 2.867e-13 | 1.586e-14 | 1.0e-08 | Eh | MB16-43-03 | True |
| gfn2 | gradient | 3.989e-14 | 3.003e-15 | 1.0e-07 | Eh/Bohr | MB16-43-13 | True |
| gfn2 | virial | 1.164e-13 | 1.197e-14 | 1.0e-06 | Eh | MB16-43-13 | True |
| gfn2 | charges | 8.684e-13 | 4.809e-14 | 1.0e-06 | e | MB16-43-03 | True |
| gfn2 | dipole | 9.030e-12 | 1.013e-12 | 1.0e-05 | e Bohr | MB16-43-03 | True |
| gfn2 | quadrupole | 4.681e-11 | 3.852e-12 | 1.0e-04 | e Bohr^2 | MB16-43-03 | True |
| ipea1 | energy | 2.842e-14 | 1.035e-14 | 1.0e-08 | Eh | MB16-43-34 | True |
| ipea1 | energies | 6.725e-14 | 4.928e-15 | 1.0e-08 | Eh | MB16-43-30 | True |
| ipea1 | gradient | 9.819e-15 | 6.441e-16 | 1.0e-07 | Eh/Bohr | MB16-43-30 | True |
| ipea1 | virial | 1.235e-14 | 2.583e-15 | 1.0e-06 | Eh | MB16-43-13 | True |
| ipea1 | charges | 1.662e-13 | 1.212e-14 | 1.0e-06 | e | MB16-43-30 | True |
| ipea1 | dipole | 1.971e-12 | 2.069e-13 | 1.0e-05 | e Bohr | MB16-43-30 | True |
| ipea1 | quadrupole | 1.099e-11 | 9.972e-13 | 1.0e-04 | e Bohr^2 | MB16-43-30 | True |

Data: [results.json](results.json), [summary.csv](summary.csv), [cases.csv](cases.csv).
