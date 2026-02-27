package soroban

import (
	"encoding/json"
	"fmt"
)

// EventCompatPayload is a minimal normalized representation used by indexers/SDK code.
// It intentionally keeps only required cross-version fields.
type EventCompatPayload struct {
	Version      uint32                     `json:"version"`
	Amount       int64                      `json:"amount,omitempty"`
	Jurisdiction *JurisdictionCompatPayload `json:"jurisdiction,omitempty"`
}

// JurisdictionCompatPayload captures optional jurisdiction tags/flags emitted by
// updated escrow/program contracts.
type JurisdictionCompatPayload struct {
	Tag           string `json:"tag,omitempty"`
	RequiresKYC   bool   `json:"requires_kyc,omitempty"`
	EnforceLimits bool   `json:"enforce_limits,omitempty"`
	LockPaused    bool   `json:"lock_paused,omitempty"`
	ReleasePaused bool   `json:"release_paused,omitempty"`
	RefundPaused  bool   `json:"refund_paused,omitempty"`
	MaxAmount     int64  `json:"max_amount,omitempty"`
}

// ParseEventCompatPayload parses both legacy (v1, unversioned) and version-tagged payloads.
// Unknown/newer versions are accepted as long as required fields are present.
func ParseEventCompatPayload(raw []byte) (*EventCompatPayload, error) {
	var payload map[string]json.RawMessage
	if err := json.Unmarshal(raw, &payload); err != nil {
		return nil, fmt.Errorf("decode event payload: %w", err)
	}

	out := &EventCompatPayload{Version: 1}
	if rawVersion, ok := payload["version"]; ok {
		if err := json.Unmarshal(rawVersion, &out.Version); err != nil {
			return nil, fmt.Errorf("decode version: %w", err)
		}
	}

	if rawAmount, ok := payload["amount"]; ok {
		if err := json.Unmarshal(rawAmount, &out.Amount); err != nil {
			return nil, fmt.Errorf("decode amount: %w", err)
		}
	}

	// Newer payload shape: nested jurisdiction object.
	if rawJurisdiction, ok := payload["jurisdiction"]; ok {
		var jurisdiction JurisdictionCompatPayload
		if err := json.Unmarshal(rawJurisdiction, &jurisdiction); err != nil {
			return nil, fmt.Errorf("decode jurisdiction: %w", err)
		}
		out.Jurisdiction = &jurisdiction
	}

	// Legacy/flat payload shape compatibility.
	if rawTag, ok := payload["jurisdiction_tag"]; ok {
		if out.Jurisdiction == nil {
			out.Jurisdiction = &JurisdictionCompatPayload{}
		}
		if err := json.Unmarshal(rawTag, &out.Jurisdiction.Tag); err != nil {
			return nil, fmt.Errorf("decode jurisdiction_tag: %w", err)
		}
	}
	if rawRequiresKYC, ok := payload["requires_kyc"]; ok {
		if out.Jurisdiction == nil {
			out.Jurisdiction = &JurisdictionCompatPayload{}
		}
		if err := json.Unmarshal(rawRequiresKYC, &out.Jurisdiction.RequiresKYC); err != nil {
			return nil, fmt.Errorf("decode requires_kyc: %w", err)
		}
	}
	if rawEnforceLimits, ok := payload["enforce_limits"]; ok {
		if out.Jurisdiction == nil {
			out.Jurisdiction = &JurisdictionCompatPayload{}
		}
		if err := json.Unmarshal(rawEnforceLimits, &out.Jurisdiction.EnforceLimits); err != nil {
			return nil, fmt.Errorf("decode enforce_limits: %w", err)
		}
	}
	if rawLockPaused, ok := payload["lock_paused"]; ok {
		if out.Jurisdiction == nil {
			out.Jurisdiction = &JurisdictionCompatPayload{}
		}
		if err := json.Unmarshal(rawLockPaused, &out.Jurisdiction.LockPaused); err != nil {
			return nil, fmt.Errorf("decode lock_paused: %w", err)
		}
	}
	if rawReleasePaused, ok := payload["release_paused"]; ok {
		if out.Jurisdiction == nil {
			out.Jurisdiction = &JurisdictionCompatPayload{}
		}
		if err := json.Unmarshal(rawReleasePaused, &out.Jurisdiction.ReleasePaused); err != nil {
			return nil, fmt.Errorf("decode release_paused: %w", err)
		}
	}
	if rawRefundPaused, ok := payload["refund_paused"]; ok {
		if out.Jurisdiction == nil {
			out.Jurisdiction = &JurisdictionCompatPayload{}
		}
		if err := json.Unmarshal(rawRefundPaused, &out.Jurisdiction.RefundPaused); err != nil {
			return nil, fmt.Errorf("decode refund_paused: %w", err)
		}
	}
	if rawMaxAmount, ok := payload["max_amount"]; ok {
		if out.Jurisdiction == nil {
			out.Jurisdiction = &JurisdictionCompatPayload{}
		}
		if err := json.Unmarshal(rawMaxAmount, &out.Jurisdiction.MaxAmount); err != nil {
			return nil, fmt.Errorf("decode max_amount: %w", err)
		}
	}

	if _, hasAmount := payload["amount"]; !hasAmount {
		return nil, fmt.Errorf("missing required field: amount")
	}

	return out, nil
}
