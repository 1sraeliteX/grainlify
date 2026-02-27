-- On-chain smart-contract events table.
-- Stores ingested Soroban events so that backend analytics can join
-- on-chain data (escrow locks, releases, payouts, governance) with
-- off-chain GitHub/project data without brittle ETL transformations.
--
-- Field names intentionally match the v2 event schema defined in
-- contracts/EVENT_SCHEMA.md so indexers can INSERT without renaming.

CREATE TABLE IF NOT EXISTS onchain_events (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ledger_sequence BIGINT       NOT NULL,
    tx_hash         TEXT         NOT NULL,
    contract_id     TEXT         NOT NULL,
    topic           TEXT         NOT NULL,          -- e.g. "f_lock", "PrgInit"
    version         INT          NOT NULL DEFAULT 2,
    event_timestamp BIGINT       NOT NULL,          -- ledger timestamp (unix)
    payload         JSONB        NOT NULL DEFAULT '{}',
    received_at     TIMESTAMPTZ  NOT NULL DEFAULT NOW(),

    -- Denormalised fields for the most common query patterns.
    bounty_id       BIGINT,                         -- NULL for program-escrow events
    program_id      TEXT,                            -- NULL for bounty-escrow events
    amount          NUMERIC(78,0),                   -- i128 value (stroops)
    recipient       TEXT
);

-- Indexes for common analytics queries.
CREATE INDEX IF NOT EXISTS idx_onchain_events_topic       ON onchain_events (topic);
CREATE INDEX IF NOT EXISTS idx_onchain_events_contract    ON onchain_events (contract_id);
CREATE INDEX IF NOT EXISTS idx_onchain_events_bounty      ON onchain_events (bounty_id)  WHERE bounty_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_onchain_events_program     ON onchain_events (program_id) WHERE program_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_onchain_events_received    ON onchain_events (received_at);
CREATE INDEX IF NOT EXISTS idx_onchain_events_ledger      ON onchain_events (ledger_sequence);
