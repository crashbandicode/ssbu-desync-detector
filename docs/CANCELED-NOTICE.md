# Canceled notice

2026-11-07 (committed in 0.4.0, `ca0c149`). Written before the matcher change. Playbook `3f96569` (clone already at that
commit; `git pull --ff-only` was not required to move it). The hooked plain
route `.377a120` and the argument route `.37a2660` already edit the label
buffer. No new site. English labels from the stock 13.0.5 `data.arc`, region
2: `mnu_pallet_matching_canceled` and `mnu_online_room_inside_cancel` are
`Canceled`; `pop_local_participation_stop` is `Canceled.`;
`pop_online_penalty_cut_com_warning` contains "You disconnected in the middle
of your last online battle."

| Requirement (paraphrased) | Design | Test |
| --- | --- | --- |
| Show the detector's reason in that pop-up too | `is_disconnect_notice` accepts only those three texts. Both existing routes append the same `Reason` line. A notice with no session verdict is left unchanged; a communication-error sentence still falls back to Unknown. | `canceled_notice_and_reconnect_warning_take_the_same_reason_on_both_routes`, `a_withheld_verdict_leaves_the_canceled_notice_unchanged`, `other_cancel_wording_is_left_alone` |
| Log more about where the notice comes from | `notice_text` on the existing plain hook names the kind, the caller of `.377a120` (saved LR at FP+8), and `verdict=withheld` when no session verdict exists. No new hook. | `notice_text_names_the_kind_the_caller_and_a_withheld_verdict` |

Sharing is N/A: the sites were already this plugin's, and the change adds no
hook. The card was not written. Eden was not booted.

2026-11-13 note: in Eden on 2026-11-12, release 0.4.0 (`a31ad657`, runs `dbl-ga-02420ee` and `dbl-gb2-02420ee`)
and 0.5.0 (`proof-dc8ee988-r2`) logged `notice_text kind=canceled_period text_units=9 route=plain caller=0x1eef838
verdict=unknown` on console a, after `fault code=1 frame=3215`. No Switch log shows a `notice_text` line yet
([VERSIONS.md](VERSIONS.md)).
