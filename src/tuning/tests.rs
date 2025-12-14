use crate::prelude::*;
use crate::tuning::params::{self, SPSA_VECTOR_SIZE, TunableParams};
use crate::tuning::trace::EvalTrace;

#[test]
fn test_mobility_params_mapping() {
    let mut params = TunableParams::default();

    // Set specific values for mobility to verify they end up in the right place
    // Knight move 0 (trapped) = 100
    params.mobility_knight[0] = Score::new(100, 200);
    // Knight move 8 (max) = 108
    params.mobility_knight[8] = Score::new(108, 208);

    // Bishop move 5 = 305
    params.mobility_bishop[5] = Score::new(305, 405);

    // Queen move 27 (max) = 527
    params.mobility_queen[27] = Score::new(527, 627);

    //  Convert to Vector
    let vec = params.to_vector();

    //  Verify Vector Size
    assert_eq!(vec.len(), SPSA_VECTOR_SIZE, "Vector size mismatch!");

    // Verify Knight Mapping via Trace
    // We want to find where Knight Move 0 lives in the vector
    let trace_idx_k0 = params::MOBILITY_KNIGHT_START;
    let vec_idx_k0 = EvalTrace::map_feature_to_spsa_index(trace_idx_k0);

    assert_eq!(vec[vec_idx_k0], 100.0, "Knight Move 0 MG mismatch");
    assert_eq!(vec[vec_idx_k0 + 1], 200.0, "Knight Move 0 EG mismatch");

    // Verify Knight Max
    let trace_idx_k8 = params::MOBILITY_KNIGHT_START + 8;
    let vec_idx_k8 = EvalTrace::map_feature_to_spsa_index(trace_idx_k8);
    assert_eq!(vec[vec_idx_k8], 108.0, "Knight Move 8 MG mismatch");

    // Verify Bishop Mapping
    let trace_idx_b5 = params::MOBILITY_BISHOP_START + 5;
    let vec_idx_b5 = EvalTrace::map_feature_to_spsa_index(trace_idx_b5);
    assert_eq!(vec[vec_idx_b5], 305.0, "Bishop Move 5 MG mismatch");

    // Verify Queen Max Mapping
    let trace_idx_q27 = params::MOBILITY_QUEEN_START + 27;
    let vec_idx_q27 = EvalTrace::map_feature_to_spsa_index(trace_idx_q27);
    assert_eq!(vec[vec_idx_q27], 527.0, "Queen Move 27 MG mismatch");
}

#[test]
fn test_trace_feature_bounds() {
    // Ensure that the constants in params.rs line up perfectly

    // Check Knight Start
    let expected_knight_start = params::PST_START + params::NUM_PST_PARAMS;
    assert_eq!(
        params::MOBILITY_KNIGHT_START,
        expected_knight_start,
        "Knight Start Offset is wrong"
    );

    // Check Bishop Start
    let expected_bishop_start = params::MOBILITY_KNIGHT_START + params::KNIGHT_MAX;
    assert_eq!(
        params::MOBILITY_BISHOP_START,
        expected_bishop_start,
        "Bishop Start Offset is wrong"
    );

    // Check Total Features
    let expected_total = params::MOBILITY_QUEEN_START + params::QUEEN_MAX;
    assert_eq!(
        params::NUM_TRACE_FEATURES,
        expected_total,
        "Total Trace Features count is wrong"
    );
}

#[test]
fn test_round_trip_conversion() {
    // Verify that to_vector -> from_vector preserves data
    let mut original = TunableParams::default();
    original.mobility_rook[7] = Score::new(123, 456);
    original.tempo_bonus = Score::new(50, 50);

    let vec = original.to_vector();
    let recovered = TunableParams::from_vector(&vec);

    assert_eq!(original.mobility_rook[7], recovered.mobility_rook[7]);
    assert_eq!(original.tempo_bonus, recovered.tempo_bonus);
}

#[test]
fn test_pst_mapping_correctness() {
    let mut params = TunableParams::zeros();

    // Set a unique value for White King on E1 (Index 4)
    // 4 is the index for E1 (File 4, Rank 0)
    let e1 = 4;
    let unique_score = Score::new(999, 888);

    // Calculate the flat index for King (5th piece) on square 4
    let flat_idx = (Piece::King.index() * 64) + e1;
    params.psts[flat_idx] = unique_score;

    let vec = params.to_vector();

    // Calculate expected SPSA index
    // Trace Index = PST_START + flat_idx
    let trace_idx = params::PST_START + flat_idx;
    let spsa_idx = EvalTrace::map_feature_to_spsa_index(trace_idx);

    assert_eq!(vec[spsa_idx], 999.0, "PST MG value mismatch");
    assert_eq!(vec[spsa_idx + 1], 888.0, "PST EG value mismatch");
}

#[test]
fn test_passed_pawn_indexing() {
    // Passed pawns are stored in indices 15..22 in params.rs (PASSED_PAWN_START)
    // They correspond to ranks 0..7 relative to the side.

    let rank = 6;
    let param_idx = params::PASSED_PAWN_START + rank;

    // In Trace, it should map to the same relative offset
    // map_feature_to_spsa_index handles the logic:
    // i if (PASSED_PAWN_START..PASSED_PAWN_START + 8).contains(&i)

    let trace_idx = crate::tuning::trace::PASSED_PAWN_START + rank;

    let calculated_spsa_idx = EvalTrace::map_feature_to_spsa_index(trace_idx);
    let expected_spsa_idx = param_idx * 2;

    assert_eq!(
        calculated_spsa_idx, expected_spsa_idx,
        "Passed Pawn Rank {} mapping failed",
        rank
    );
}

#[test]
fn test_texel_entry_phase_normalization() {
    use crate::tuning::texel::TexelEntry;

    let start_phase = 0; // Midgame
    let end_phase = 256; // Endgame
    let mid_phase = 128;

    let norm_start = start_phase as f64 / 256.0;
    let norm_end = end_phase as f64 / 256.0;
    let norm_mid = mid_phase as f64 / 256.0;

    assert_eq!(norm_start, 0.0);
    assert_eq!(norm_end, 1.0);
    assert_eq!(norm_mid, 0.5);

    let entry = TexelEntry {
        trace: EvalTrace::default(),
        fixed_score: Score::new(100, 200), // MG 100, EG 200
        result: 0.5,
        phase: 0.5, // Exact middle
    };

    let weights = vec![0.0; 1000];
    let feature_map = vec![0; 1000];

    let eval = entry.evaluate(&weights, &feature_map);

    assert!(
        (eval - 150.0).abs() < 1e-6,
        "Texel Phase Interpolation incorrect"
    );
}
