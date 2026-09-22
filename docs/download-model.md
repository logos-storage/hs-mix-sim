# Anonymous download model

A user performs one bounded download. The model estimates the total packet paths required for data, SURB supply, and control traffic, then repeatedly samples paths with one persistent session selector. The session is compromised if any sampled path is fully malicious.

## From file size to packet count

To model the anonymous download over the transport layer as accurately as possible, we need to model the chunking, erasure coding, SURB supply, and transport/control overhead traffic from the transport layer protocol.

The model needs to define the user behaviour: how many packets are sent and when?
We assume all packets in a session are sent in a short time and so we are not considering churn here. We can approximate the number of packets based on a few parameters:
- $F$ be the download file(s) size in bytes
- $P(L)$ be the usable Sphinx payload ($\delta$) size for a path of $L$ hops
- $H_T$ be the transport-layer overhead added to each data chunk. This is a constant value.
- $R(L)=P(L)-H_T$ be the amount of file data that fits in one transport chunk.
- $r = \frac{n}{k}$ where $1 \leq r \leq 2$ be the erasure-coding redundancy ratio
- $S_{\mathrm{SURB}}(L)$ be the size of one $L$-hop SURB.
- $N_\texttt{session}$ be the number of sphinx packets needed to complete the session, i.e. the number of paths selected by the downloader for both forward and backward (SURBs) packets.

The number of chunks needed for the file is basically a function of $F$ and $L$:

$$
K(F,L) = \left\lceil \frac{F}{R(L)} \right\rceil.
$$

After adding erasure-coding redundancy, the number of provider-to-downloader data packets is

$$
N_D(F,L,r) =
\left\lceil
rK(F,L)
\right\rceil
$$

Since the downloader is anonymous, each return packet from the provider requires one SURB. Therefore, approximately $N_{\mathrm{SURB}}=N_D$ SURBs must be supplied to the provider.

The number of SURBs that fit inside one forward transport packet is:

$$
C_{\mathrm{SURB}}(L) =
\left\lfloor
\frac{P(L)-H_T}
{S_{\mathrm{SURB}}(L)}
\right\rfloor.
$$

The number of downloader-to-provider packets needed to supply these SURBs is then:

$$
N_F =
\left\lceil
\frac{N_D}
{C_{\mathrm{SURB}}(L)}
\right\rceil.
$$

We can additionally assume a $5\%$ overhead for transport-control traffic such as requests, acknowledgements, session control, and SURB-management messages that are not already captured above. The total number of Mix packets in the session is therefore approximated as:

$$
N_{\texttt{session}} =
\left\lceil
1.05\left(N_D+N_F\right)
\right\rceil
$$

We can then substitute the previous terms and compute the number of packets needed for the simulated session as a function that depends on:
- the download file size
- number of hops in the mix path
- the erasure coding redundancy ratio/rate

$$
N_{\texttt{session}}(F,L,r) =
\left\lceil
1.05 \left( \left\lceil r \left\lceil \frac{F}{R(L)}
\right\rceil \right\rceil
+
\left\lceil \frac{ \left\lceil r \left\lceil
F/R(L)
\right\rceil \right\rceil}{C_{\mathrm{SURB}}(L)}
\right\rceil \right) \right\rceil
$$

We assume the erasure-coding redundancy is sufficient and so we don't account for retransmissions. If EC reconstruction fails, the session is considered failed and a new session would be required.

The computed approximate number of packets $N_{\texttt{session}}$ will then be used in the simulation to evaluate the different path selection strategies.

For an anonymous download, we define the session as compromised if at least one fully malicious path is selected at any point during the session.

For a 1 MiB file, 4,608-byte packet size, and three hops: $H=400$, $C=4166$, $N_0=252$, $N_D=378$, $C_{\mathrm{SURB}}=8$, $N_F=48$, and $N=448$ packet paths.

## Session selectors
these are supported:
- `random`
- `k-hf`
- `k-w`
- `alpha-sticky` 

## Session profiles

Download profiles select K/W parameters:

| Profile | Hops $`L`$ | Fixed pools $`L_f`$  | Candidates per pool $`K`$  | random positions per packet |
| --- |------:|-------------:|---------------------:|----------------------------:|
| `LITE` |     3 |            1 |                    5 |                           2 |
| `STANDARD` |     3 |            2 |                    5 |                           1 |
| `STRICT` |     4 |            3 |                    3 |                           1 |
3-3-3-R`) | 4 | 3 | 3 |