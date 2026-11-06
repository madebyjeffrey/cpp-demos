#[path = "../examples/resources/workload.rs"]
mod workload;

#[test]
fn collection_churn_and_long_edit_sessions_keep_realization_and_caches_bounded() {
    let report = workload::run(10_000, 32, 1_100).unwrap();
    assert_eq!(
        (report.rows, report.visits, report.edits),
        (10_000, 32, 1_100)
    );
    assert!(
        report.paragraphs >= 500,
        "exercise paragraph eviction: {report:?}"
    );
    assert_eq!(report.images, 128, "exercise image eviction: {report:?}");
    println!("{report:#?}");
}
