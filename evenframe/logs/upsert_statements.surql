
UPSERT appointment:1 MERGE { recurrence_rule: (IF recurrence_rule != NULL THEN { additional_instances: null } ELSE NULL END) };
UPSERT appointment:2 MERGE { recurrence_rule: (IF recurrence_rule != NULL THEN { additional_instances: [r'appointment:13', r'appointment:13', r'appointment:2', r'appointment:8', r'appointment:13', r'appointment:11', r'appointment:1'] } ELSE NULL END) };
UPSERT appointment:3 MERGE { recurrence_rule: (IF recurrence_rule != NULL THEN { additional_instances: null } ELSE NULL END) };
UPSERT appointment:4 MERGE { recurrence_rule: (IF recurrence_rule != NULL THEN { additional_instances: [r'appointment:1', r'appointment:11', r'appointment:13', r'appointment:10', r'appointment:8', r'appointment:13', r'appointment:2', r'appointment:7'] } ELSE NULL END) };
UPSERT appointment:5 MERGE { recurrence_rule: (IF recurrence_rule != NULL THEN { additional_instances: [r'appointment:4', r'appointment:6'] } ELSE NULL END) };
UPSERT appointment:6 MERGE { recurrence_rule: (IF recurrence_rule != NULL THEN { additional_instances: [r'appointment:9', r'appointment:5', r'appointment:1', r'appointment:12', r'appointment:7', r'appointment:2', r'appointment:2'] } ELSE NULL END) };
UPSERT appointment:7 MERGE { recurrence_rule: (IF recurrence_rule != NULL THEN { additional_instances: [r'appointment:12', r'appointment:5', r'appointment:5', r'appointment:3', r'appointment:1'] } ELSE NULL END) };
UPSERT appointment:8 MERGE { recurrence_rule: (IF recurrence_rule != NULL THEN { additional_instances: null } ELSE NULL END) };
UPSERT appointment:9 MERGE { recurrence_rule: (IF recurrence_rule != NULL THEN { additional_instances: null } ELSE NULL END) };
UPSERT appointment:10 MERGE { recurrence_rule: (IF recurrence_rule != NULL THEN { additional_instances: null } ELSE NULL END) };
UPSERT appointment:11 MERGE { recurrence_rule: (IF recurrence_rule != NULL THEN { additional_instances: null } ELSE NULL END) };
UPSERT appointment:12 MERGE { recurrence_rule: (IF recurrence_rule != NULL THEN { additional_instances: null } ELSE NULL END) };
UPSERT appointment:13 MERGE { recurrence_rule: (IF recurrence_rule != NULL THEN { additional_instances: [r'appointment:12', r'appointment:7', r'appointment:6', r'appointment:13', r'appointment:4', r'appointment:6'] } ELSE NULL END) };

