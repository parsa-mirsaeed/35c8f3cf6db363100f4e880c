# Third-party notices

The appliance contains unmodified third-party container images selected by the
pinned production Compose topology. Their exact source references and registry
digests are recorded in `manifests/release-manifest.json`; their package
inventories and declared licences are recorded in `sbom/images/*.spdx.json`.

The optional local embedding profile packages `BAAI/bge-small-en-v1.5` at commit
`5c38ec7c405ec4b44b94cc5a9bb96e735b38267a`. Its model card declares the MIT
licence. The model files and their checksums are recorded under
`models/local-bge-v1/`.

The bundle may include images from projects such as Caddy, Supabase, PostgreSQL,
PostgREST, GoTrue, Realtime, Storage, Supavisor, Qdrant, and Hugging Face Text
Embeddings Inference. This notice is not a substitute for the licence texts or
attribution requirements reported by the generated SBOMs. Release operators must
review those SBOMs and the upstream licences before distribution.

EduTalent source and custom images remain governed by the repository `LICENSE`.
No third-party notice grants rights to EduTalent source code.
