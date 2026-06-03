/*!
 * Copyright 2025 Release Workshop Ltd
 * Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
 * See the LICENSE file in the project root for details.
 */

mod evaluate;

pub use evaluate::{
    evaluate_flag, evaluate_rule, find_flag_index, get_property, rollout_bucket, user_id,
    EvaluationAttributes, RuleEvaluation,
};
