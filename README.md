# Subatomic-NG (Next Generation)

Subatomic-NG is a complete rewrite of the original Subatomic repository server software. It's designed to be cleaner, more storage efficient, and
more scalable than the original Subatomic.

> [!NOTE]
> Subatomic-NG is currently in its early experimental development phase.
> It's still currently not ready for production use.
>
> It's meant to be a proof of concept for the new design and architecture, and possibly maybe a future replacement for the original Subatomic.

## Design Paradigms

Subatomic-NG follows these key design principles:

- Keeping records of historical data (with optional redaction for some edge cases)
- Atomicity (exported data only changes after a successful transaction)
- Content-addressable storage for artifacts
- Implements efficient deduplication
- Separates concerns:
  - Object store for artifact storage
  - Database for metadata only (names, versions, package info)
- Scales horizontally with storage capacity
- Local disk storage should only be used for caching and serving current data
- Simple restore process (Backup only the database, large artifacts can be re-downloaded from object storage)

## Components

Subatomic-NG uses [Axum] for its HTTP API, [SurrealDB] for its database, and supports any [S3]-compatible object store for artifact storage.

[Axum]: https://github.com/tokio-rs/axum
[SurrealDB]: https://surrealdb.com/
[S3]: https://aws.amazon.com/s3/

## Configuration

Subatomic-NG is configured using environment variables or CLI options. You may also try calling `subatomic-ng --help` to see a list of available options.

### Extra environment variables

- `NO_UPLOAD`: If set, disables the Object store upload feature. Useful for testing.

## License

Subatomic-NG is licensed under the GPL-3.0 license. See the [LICENSE](LICENSE) file for more details.
