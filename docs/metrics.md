# Metrics and formulas

## Packet, session, and time-based

The de-anonymization likelihood (`DLM`) terms:

| Metric | Event being measured                                              |
| --- |-------------------------------------------------------------------|
| P-DLM | One sampled packet path is fully malicious.                       |
| S-DLM | At least one path in a related packet session is fully malicious. |
| T-DLM | A hidden service is identified by the adversary before time $t$.  |

[TODO: add formulas ...]