# MB16-43 native / Rust parity

Result: **PASS**. 129 of 129 pairs compared, 0 failed.

tblite 0.8.0, Linux-6.18.54.1-microsoft-standard-WSL2-x86_64-with-glibc2.35, 2026-11-09T13:57:38.688734+00:00. SAD guess, accuracy 0.01, at most 250 SCF iterations, kT = 0.000950042573563535 Eh (300 K), one OpenMP/BLAS thread.

| Method | Property | Max absolute error | RMSE | Bound | Unit | Worst molecule | Pass |
| --- | --- | ---: | ---: | ---: | --- | --- | --- |
| gfn1 | energy | 0.000e+00 | 0.000e+00 | 1.0e-08 | Eh | MB16-43-01 | True |
| gfn1 | energies | 0.000e+00 | 0.000e+00 | 1.0e-08 | Eh | MB16-43-01 | True |
| gfn1 | gradient | 0.000e+00 | 0.000e+00 | 1.0e-07 | Eh/Bohr | MB16-43-01 | True |
| gfn1 | virial | 1.665e-16 | 2.963e-17 | 1.0e-06 | Eh | MB16-43-11 | True |
| gfn1 | charges | 0.000e+00 | 0.000e+00 | 1.0e-06 | e | MB16-43-01 | True |
| gfn1 | dipole | 0.000e+00 | 0.000e+00 | 1.0e-05 | e Bohr | MB16-43-01 | True |
| gfn1 | quadrupole | 0.000e+00 | 0.000e+00 | 1.0e-04 | e Bohr^2 | MB16-43-01 | True |
| gfn2 | energy | 0.000e+00 | 0.000e+00 | 1.0e-08 | Eh | MB16-43-01 | True |
| gfn2 | energies | 0.000e+00 | 0.000e+00 | 1.0e-08 | Eh | MB16-43-01 | True |
| gfn2 | gradient | 0.000e+00 | 0.000e+00 | 1.0e-07 | Eh/Bohr | MB16-43-01 | True |
| gfn2 | virial | 1.123e-16 | 2.848e-17 | 1.0e-06 | Eh | MB16-43-06 | True |
| gfn2 | charges | 0.000e+00 | 0.000e+00 | 1.0e-06 | e | MB16-43-01 | True |
| gfn2 | dipole | 0.000e+00 | 0.000e+00 | 1.0e-05 | e Bohr | MB16-43-01 | True |
| gfn2 | quadrupole | 0.000e+00 | 0.000e+00 | 1.0e-04 | e Bohr^2 | MB16-43-01 | True |
| ipea1 | energy | 0.000e+00 | 0.000e+00 | 1.0e-08 | Eh | MB16-43-01 | True |
| ipea1 | energies | 0.000e+00 | 0.000e+00 | 1.0e-08 | Eh | MB16-43-01 | True |
| ipea1 | gradient | 0.000e+00 | 0.000e+00 | 1.0e-07 | Eh/Bohr | MB16-43-01 | True |
| ipea1 | virial | 1.145e-16 | 3.092e-17 | 1.0e-06 | Eh | MB16-43-27 | True |
| ipea1 | charges | 0.000e+00 | 0.000e+00 | 1.0e-06 | e | MB16-43-01 | True |
| ipea1 | dipole | 0.000e+00 | 0.000e+00 | 1.0e-05 | e Bohr | MB16-43-01 | True |
| ipea1 | quadrupole | 0.000e+00 | 0.000e+00 | 1.0e-04 | e Bohr^2 | MB16-43-01 | True |

Data: [results.json](results.json), [summary.csv](summary.csv), [cases.csv](cases.csv).
