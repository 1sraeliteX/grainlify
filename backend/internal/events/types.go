package events

import "encoding/json"

const (
	SubjectGitHubWebhookReceived = "github.webhook.received"

	// On-chain event subjects — emitted by the Soroban indexer pipeline.
	SubjectOnChainFundsLocked    = "onchain.funds.locked"
	SubjectOnChainFundsReleased  = "onchain.funds.released"
	SubjectOnChainFundsRefunded  = "onchain.funds.refunded"
	SubjectOnChainBatchLocked    = "onchain.batch.locked"
	SubjectOnChainBatchReleased  = "onchain.batch.released"
	SubjectOnChainPayout         = "onchain.payout"
	SubjectOnChainBatchPayout    = "onchain.batch.payout"
	SubjectOnChainProgramInit    = "onchain.program.init"
	SubjectOnChainFeeCollected   = "onchain.fee.collected"
	SubjectOnChainPauseChanged   = "onchain.pause.changed"
	SubjectOnChainEmergencyWd    = "onchain.emergency.withdraw"
	SubjectOnChainGovProposal    = "onchain.gov.proposal"
	SubjectOnChainGovVote        = "onchain.gov.vote"
	SubjectOnChainGovFinalized   = "onchain.gov.finalized"
)

type GitHubWebhookReceived struct {
	DeliveryID   string          `json:"delivery_id"`
	Event        string          `json:"event"`
	Action       string          `json:"action,omitempty"`
	RepoFullName string          `json:"repo_full_name,omitempty"`
	Payload      json.RawMessage `json:"payload"`
}

// ---------------------------------------------------------------------------
// On-chain event envelope — fields aligned with the v2 contract event schema.
// All monetary values use stroops (i128 serialised as string).
// ---------------------------------------------------------------------------

// OnChainEventEnvelope is the common wrapper for all on-chain Soroban events
// ingested by the backend. The Topic field matches the Soroban topic symbol,
// and Version must be >= 2 for v2 payloads.
type OnChainEventEnvelope struct {
	// LedgerSequence is the Stellar ledger that included the event.
	LedgerSequence uint64          `json:"ledger_sequence"`
	TxHash         string          `json:"tx_hash"`
	ContractID     string          `json:"contract_id"`
	Topic          string          `json:"topic"`
	Version        uint32          `json:"version"`
	Timestamp      uint64          `json:"timestamp"`
	Payload        json.RawMessage `json:"payload"`
}

// Bounty-escrow event payloads (align with the Rust event structs).

type OnChainFundsLocked struct {
	Version   uint32 `json:"version"`
	BountyID  uint64 `json:"bounty_id"`
	Amount    string `json:"amount"`
	Depositor string `json:"depositor"`
	Deadline  uint64 `json:"deadline"`
}

type OnChainFundsReleased struct {
	Version   uint32 `json:"version"`
	BountyID  uint64 `json:"bounty_id"`
	Amount    string `json:"amount"`
	Recipient string `json:"recipient"`
	Timestamp uint64 `json:"timestamp"`
}

type OnChainFundsRefunded struct {
	Version   uint32 `json:"version"`
	BountyID  uint64 `json:"bounty_id"`
	Amount    string `json:"amount"`
	RefundTo  string `json:"refund_to"`
	Timestamp uint64 `json:"timestamp"`
}

type OnChainFeeCollected struct {
	Version       uint32 `json:"version"`
	OperationType string `json:"operation_type"`
	Amount        string `json:"amount"`
	FeeRate       string `json:"fee_rate"`
	Recipient     string `json:"recipient"`
	Timestamp     uint64 `json:"timestamp"`
}

type OnChainBatchFundsLocked struct {
	Version     uint32 `json:"version"`
	Count       uint32 `json:"count"`
	TotalAmount string `json:"total_amount"`
	Timestamp   uint64 `json:"timestamp"`
}

type OnChainBatchFundsReleased struct {
	Version     uint32 `json:"version"`
	Count       uint32 `json:"count"`
	TotalAmount string `json:"total_amount"`
	Timestamp   uint64 `json:"timestamp"`
}

// Program-escrow event payloads.

type OnChainProgramInitialized struct {
	Version            uint32 `json:"version"`
	ProgramID          string `json:"program_id"`
	AuthorizedPayoutKey string `json:"authorized_payout_key"`
	TokenAddress       string `json:"token_address"`
	TotalFunds         string `json:"total_funds"`
}

type OnChainPayout struct {
	Version          uint32 `json:"version"`
	ProgramID        string `json:"program_id"`
	Recipient        string `json:"recipient"`
	Amount           string `json:"amount"`
	RemainingBalance string `json:"remaining_balance"`
}

type OnChainBatchPayout struct {
	Version          uint32 `json:"version"`
	ProgramID        string `json:"program_id"`
	RecipientCount   uint32 `json:"recipient_count"`
	TotalAmount      string `json:"total_amount"`
	RemainingBalance string `json:"remaining_balance"`
}





