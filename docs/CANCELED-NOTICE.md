# Canceled notice

Written before the matcher change. Playbook `3f96569` (clone already at that
commit; `git pull --ff-only` was not required to move it). The hooked plain
route `.377a120` and the argument route `.37a2660` already edit the label
buffer. No new site. English labels from the stock 13.0.5 `data.arc`, region
2: `mnu_pallet_matching_canceled` and `mnu_online_room_inside_cancel` are
`Canceled`; `pop_local_participation_stop` is `Canceled.`;
`pop_online_penalty_cut_com_warning` contains "You disconnected in the middle
of your last online battle."

| Operator clause | Design | Test |
| --- | --- | --- |
| the desync detector also need to show in that pop up as well. | `is_disconnect_notice` accepts only those three texts. Both existing routes append the same `Reason` line. A notice with no session verdict is left unchanged; a communication-error sentence still falls back to Unknown. | `canceled_notice_and_reconnect_warning_take_the_same_reason_on_both_routes`, `a_withheld_verdict_leaves_the_canceled_notice_unchanged`, `other_cancel_wording_is_left_alone` |
| Perhaps we can expand our logging. | `notice_text` on the existing plain hook names the kind, the caller of `.377a120` (saved LR at FP+8), and `verdict=withheld` when no session verdict exists. No new hook. | `notice_text_names_the_kind_the_caller_and_a_withheld_verdict` |

Sharing is N/A: the sites were already this plugin's, and the change adds no
hook. The card was not written. Eden was not booted.
