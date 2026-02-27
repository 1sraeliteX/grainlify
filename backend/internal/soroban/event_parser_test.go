package soroban

import "testing"

func TestParseEventCompatPayload_LegacyV1WithoutVersionTag(t *testing.T) {
	raw := []byte(`{"amount": 1500, "program_id": "hack-2026"}`)

	parsed, err := ParseEventCompatPayload(raw)
	if err != nil {
		t.Fatalf("ParseEventCompatPayload failed for v1 payload: %v", err)
	}
	if parsed.Version != 1 {
		t.Fatalf("expected default version 1 for legacy payload, got %d", parsed.Version)
	}
	if parsed.Amount != 1500 {
		t.Fatalf("expected amount 1500, got %d", parsed.Amount)
	}
}

func TestParseEventCompatPayload_VersionTaggedV2(t *testing.T) {
	raw := []byte(`{"version": 2, "amount": 4200, "program_id": "hack-2026", "extra":"ignored"}`)

	parsed, err := ParseEventCompatPayload(raw)
	if err != nil {
		t.Fatalf("ParseEventCompatPayload failed for v2 payload: %v", err)
	}
	if parsed.Version != 2 {
		t.Fatalf("expected version 2, got %d", parsed.Version)
	}
	if parsed.Amount != 4200 {
		t.Fatalf("expected amount 4200, got %d", parsed.Amount)
	}
}

func TestParseEventCompatPayload_NewerVersionStillParsesRequiredFields(t *testing.T) {
	raw := []byte(`{"version": 3, "amount": 999, "new_field":{"nested":true}}`)

	parsed, err := ParseEventCompatPayload(raw)
	if err != nil {
		t.Fatalf("ParseEventCompatPayload failed for newer payload: %v", err)
	}
	if parsed.Version != 3 {
		t.Fatalf("expected version 3, got %d", parsed.Version)
	}
	if parsed.Amount != 999 {
		t.Fatalf("expected amount 999, got %d", parsed.Amount)
	}
}

func TestParseEventCompatPayload_WithNestedJurisdictionConfig(t *testing.T) {
	raw := []byte(`{
		"version": 2,
		"amount": 4200,
		"jurisdiction": {
			"tag": "EU-only",
			"requires_kyc": true,
			"enforce_limits": true,
			"lock_paused": false,
			"release_paused": false,
			"refund_paused": true,
			"max_amount": 7000
		}
	}`)

	parsed, err := ParseEventCompatPayload(raw)
	if err != nil {
		t.Fatalf("ParseEventCompatPayload failed: %v", err)
	}
	if parsed.Jurisdiction == nil {
		t.Fatalf("expected jurisdiction payload to be present")
	}
	if parsed.Jurisdiction.Tag != "EU-only" {
		t.Fatalf("expected jurisdiction tag EU-only, got %q", parsed.Jurisdiction.Tag)
	}
	if !parsed.Jurisdiction.RequiresKYC {
		t.Fatalf("expected requires_kyc=true")
	}
	if !parsed.Jurisdiction.EnforceLimits {
		t.Fatalf("expected enforce_limits=true")
	}
	if !parsed.Jurisdiction.RefundPaused {
		t.Fatalf("expected refund_paused=true")
	}
	if parsed.Jurisdiction.MaxAmount != 7000 {
		t.Fatalf("expected max_amount 7000, got %d", parsed.Jurisdiction.MaxAmount)
	}
}

func TestParseEventCompatPayload_WithFlatJurisdictionFields(t *testing.T) {
	raw := []byte(`{
		"version": 2,
		"amount": 1500,
		"jurisdiction_tag": "US-only",
		"requires_kyc": false,
		"enforce_limits": false,
		"lock_paused": true,
		"release_paused": false,
		"refund_paused": false,
		"max_amount": 2000
	}`)

	parsed, err := ParseEventCompatPayload(raw)
	if err != nil {
		t.Fatalf("ParseEventCompatPayload failed: %v", err)
	}
	if parsed.Jurisdiction == nil {
		t.Fatalf("expected jurisdiction payload to be present")
	}
	if parsed.Jurisdiction.Tag != "US-only" {
		t.Fatalf("expected jurisdiction tag US-only, got %q", parsed.Jurisdiction.Tag)
	}
	if parsed.Jurisdiction.EnforceLimits {
		t.Fatalf("expected enforce_limits=false")
	}
	if !parsed.Jurisdiction.LockPaused {
		t.Fatalf("expected lock_paused=true")
	}
}

func TestParseEventCompatPayload_GenericEventWithoutJurisdiction(t *testing.T) {
	raw := []byte(`{"version": 2, "amount": 777}`)

	parsed, err := ParseEventCompatPayload(raw)
	if err != nil {
		t.Fatalf("ParseEventCompatPayload failed: %v", err)
	}
	if parsed.Jurisdiction != nil {
		t.Fatalf("expected nil jurisdiction for generic payload")
	}
}
