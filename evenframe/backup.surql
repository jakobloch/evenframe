-- ------------------------------
-- OPTION
-- ------------------------------

OPTION IMPORT;

-- ------------------------------
-- ACCESSES
-- ------------------------------

DEFINE ACCESS user ON DATABASE TYPE RECORD SIGNUP (CREATE user SET email = $email, password = crypto::argon2::generate($password)) SIGNIN (SELECT * FROM user WHERE email = $email AND crypto::argon2::compare(password, $password)) WITH JWT ALGORITHM HS512 KEY 'rBsTqB7WfMacT8n9y0eytwkK0EUFtpcaMBHAbSqWptcBVlpS0fptZfRGqpu9kxzwvHDoZ0Xa1Q7IRWjlqpvnMndMbJpZqkbjJrX7M9jGauA7aOKBtHMAETBGSEVRrjzY' WITH ISSUER KEY 'rBsTqB7WfMacT8n9y0eytwkK0EUFtpcaMBHAbSqWptcBVlpS0fptZfRGqpu9kxzwvHDoZ0Xa1Q7IRWjlqpvnMndMbJpZqkbjJrX7M9jGauA7aOKBtHMAETBGSEVRrjzY' DURATION FOR TOKEN 15m, FOR SESSION 6h;

-- ------------------------------
-- FUNCTIONS
-- ------------------------------

DEFINE FUNCTION fn::calc_order_total($prices: array<decimal>, $quantities: array<int>, $discount: decimal) { RETURN array::fold(array::transpose([$prices, $quantities]), 0, |$a: any, $b: any| $a + math::product($b)) - $discount; } PERMISSIONS FULL;
DEFINE FUNCTION fn::is_absolute_string($str: string) -> bool { RETURN $str.matches(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/); } PERMISSIONS FULL;

-- ------------------------------
-- TABLE: account
-- ------------------------------

DEFINE TABLE account TYPE NORMAL SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Account') = true;

DEFINE FIELD account_name ON account TYPE { company_name: string } | { first_name: string, last_name: string } DEFAULT { company_name: '' } PERMISSIONS FULL;
DEFINE FIELD account_type ON account TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD colors ON account TYPE { active: string, hover: string, main: string } DEFAULT { active: '', hover: '', main: '' } PERMISSIONS FULL;
DEFINE FIELD custom_fields ON account FLEXIBLE TYPE object DEFAULT {  } PERMISSIONS FULL;
DEFINE FIELD custom_fields[*] ON account TYPE string PERMISSIONS FULL;
DEFINE FIELD date_added ON account TYPE datetime DEFAULT d'2024-01-01T00:00:00Z' PERMISSIONS FULL;
DEFINE FIELD email ON account TYPE { domain_name: string, local: string, top_level_domain: string } DEFAULT { domain_name: '', local: '', top_level_domain: '' } PERMISSIONS FULL;
DEFINE FIELD has_alert ON account TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD is_tax_exempt ON account TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD lead_source ON account TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD memo ON account TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD needs_review ON account TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD payment_terms ON account TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD phones ON account TYPE array<{ can_call: bool, can_text: bool, main: bool, number: string, phone_type: string }> DEFAULT [] PERMISSIONS FULL;
DEFINE FIELD phones[*] ON account TYPE { can_call: bool, can_text: bool, main: bool, number: string, phone_type: string } PERMISSIONS FULL;
DEFINE FIELD sector ON account TYPE 'Residential' | 'Commercial' DEFAULT 'Residential' PERMISSIONS FULL;
DEFINE FIELD site ON account TYPE record<site> DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD subtype ON account TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD tags ON account TYPE array<string> DEFAULT [] PERMISSIONS FULL;
DEFINE FIELD tags[*] ON account TYPE string PERMISSIONS FULL;
DEFINE FIELD tax_rate ON account TYPE record<tax_rate> DEFAULT NULL PERMISSIONS FULL;



-- ------------------------------
-- TABLE DATA: account
-- ------------------------------


-- ------------------------------
-- TABLE: appointment
-- ------------------------------

DEFINE TABLE appointment TYPE ANY SCHEMALESS PERMISSIONS NONE;




-- ------------------------------
-- TABLE DATA: appointment
-- ------------------------------

INSERT [ { all_day: false, begins: d'2025-08-09T02:22:11.513503Z', colors: { active: 'LKWYKmdg', hover: 'TOSe2Gx2', main: '5gg0iwk8' }, description: NULL, duration: 13h47m30s691ms343µs759ns, employees: [employee:31, employee:21, employee:11, employee:6, employee:21, employee:21, employee:18, employee:3], id: appointment:1, location: site:22, multi_day: true, offset_ms: 34, recurrence_rule: NULL, status: 'OnDeck', timezone: 'America/Merida', title: '9c79dPYB' }, { all_day: false, begins: d'2025-08-09T02:22:11.513924Z', colors: { active: 'x9HFE5oG', hover: 'QTLT7Syu', main: 'KRxsfDkj' }, description: 'wBEeZiFN', duration: 13h5m33s63ms980µs947ns, employees: [employee:19, employee:8, employee:29, employee:27, employee:11, employee:30, employee:25], id: appointment:2, location: site:8, multi_day: false, offset_ms: 83, recurrence_rule: NULL, status: 'Scheduled', timezone: 'America/Santa_Isabel', title: 'HJeQa2oO' }, { all_day: true, begins: d'2025-08-09T02:22:11.514302Z', colors: { active: '5M6xRWBu', hover: '9p7Ciee5', main: '2oNNUF07' }, description: NULL, duration: 21h7m32s271ms142µs946ns, employees: [employee:10, employee:21, employee:26, employee:5, employee:19, employee:19, employee:3, employee:6], id: appointment:3, location: site:43, multi_day: false, offset_ms: 7, recurrence_rule: NULL, status: 'OnDeck', timezone: 'EST', title: 'bHkhbz49' }, { all_day: false, begins: d'2025-08-09T02:22:11.514623Z', colors: { active: 'ChlZZgfP', hover: 'VNPjKMQD', main: 'PA0S6oeS' }, description: NULL, duration: 15h33m35s306ms384µs713ns, employees: [employee:6, employee:2, employee:21], id: appointment:4, location: site:16, multi_day: true, offset_ms: 60, recurrence_rule: NULL, status: 'Scheduled', timezone: 'Asia/Jayapura', title: 'XRY2zDWR' }, { all_day: true, begins: d'2025-08-09T02:22:11.514794Z', colors: { active: '0tuhdsPG', hover: 'CMIfDsRd', main: 's8jR3MuT' }, description: NULL, duration: 14h59m58s745ms881µs664ns, employees: [employee:22, employee:2, employee:18], id: appointment:5, location: site:1, multi_day: false, offset_ms: 4, recurrence_rule: { additional_instances: NULL, cancelled_instances: NULL, interval: { quantity_of_years: 18 }, recurrence_begins: d'2025-08-09T02:22:11.515064Z', recurrence_ends: NULL }, status: 'OnDeck', timezone: 'Asia/Thimbu', title: 'pAz7Koym' }, { all_day: false, begins: d'2025-08-09T02:22:11.515110Z', colors: { active: '0iizm59k', hover: 'Sh04KvTz', main: '3DA6mjHx' }, description: NULL, duration: 3h9m44s239ms23µs623ns, employees: [employee:25, employee:2], id: appointment:6, location: site:9, multi_day: true, offset_ms: 97, recurrence_rule: { additional_instances: [appointment:12, appointment:10, appointment:8, appointment:1, appointment:8, appointment:7, appointment:3], cancelled_instances: ['jH6zLLqB', 'DxpLBJY9', '0hxKiGqM', 'kJZOTBm4', 'xf5p5Qqy'], interval: { day: 18, name: 'Kevrh1al', quantity_of_months: 83 }, recurrence_begins: d'2025-08-09T02:22:11.515348Z', recurrence_ends: 75 }, status: 'OnDeck', timezone: 'Asia/Tomsk', title: 'jreiBVIr' }, { all_day: false, begins: d'2025-08-09T02:22:11.515680Z', colors: { active: '92daxWII', hover: '1HoEG0nD', main: 'f8LZ3Vjq' }, description: 'Bz4Nn5tA', duration: 9h52m51s752ms192µs753ns, employees: [employee:15, employee:3, employee:29, employee:26, employee:6, employee:23, employee:14], id: appointment:7, location: site:19, multi_day: true, offset_ms: 55, recurrence_rule: NULL, status: 'Scheduled', timezone: 'America/Nuuk', title: 'uTiTXuE8' }, { all_day: false, begins: d'2025-08-09T02:22:11.515947Z', colors: { active: 'T0EklXjD', hover: 'izQDmUN7', main: 'lzpAlidk' }, description: NULL, duration: 15h40m28s37ms545µs87ns, employees: [employee:4, employee:18, employee:3], id: appointment:8, location: site:50, multi_day: true, offset_ms: 40, recurrence_rule: { additional_instances: [appointment:8, appointment:10, appointment:5, appointment:4], cancelled_instances: ['KpdIYkrO', '2PDCp2rU'], interval: { quantity_of_years: 72 }, recurrence_begins: d'2025-08-09T02:22:11.516230Z', recurrence_ends: NULL }, status: 'Scheduled', timezone: 'Africa/Timbuktu', title: '3RBdKsvk' }, { all_day: true, begins: d'2025-08-09T02:22:11.516409Z', colors: { active: 'IeveGR5W', hover: 'twXfdlzT', main: '5MKWhF9n' }, description: '2BpyS92y', duration: 2h29m33s425ms411µs5ns, employees: [employee:27, employee:7, employee:12, employee:1, employee:32, employee:13, employee:11], id: appointment:9, location: site:3, multi_day: false, offset_ms: 96, recurrence_rule: { additional_instances: NULL, cancelled_instances: NULL, interval: { quantity_of_weeks: 96, weekdays: ['Sunday', 'Wednesday', 'Monday', 'Monday', 'Wednesday', 'Thursday', 'Monday', 'Thursday', 'Sunday'] }, recurrence_begins: d'2025-08-09T02:22:11.516998Z', recurrence_ends: NULL }, status: 'Scheduled', timezone: 'Etc/GMT-13', title: 'eEgFz2pd' }, { all_day: true, begins: d'2025-08-09T02:22:11.517058Z', colors: { active: 'R8MaELTd', hover: 'mDY18oE3', main: '1LYo6wMb' }, description: NULL, duration: 20h32m10s572ms48µs266ns, employees: [employee:5, employee:26, employee:13, employee:9, employee:20, employee:14, employee:4, employee:27, employee:21], id: appointment:10, location: site:31, multi_day: false, offset_ms: 14, recurrence_rule: NULL, status: 'Scheduled', timezone: 'Europe/Berlin', title: 'MzgEEOBJ' }, { all_day: false, begins: d'2025-08-09T02:22:11.517396Z', colors: { active: '8mWgf5L2', hover: 'DZRvTAMz', main: 'wxyTTaCg' }, description: NULL, duration: 9h56m16s361ms274µs167ns, employees: [employee:8, employee:17, employee:27, employee:9, employee:23, employee:1], id: appointment:11, location: site:16, multi_day: true, offset_ms: 16, recurrence_rule: { additional_instances: [appointment:5, appointment:9, appointment:11, appointment:4, appointment:8, appointment:3, appointment:2, appointment:3, appointment:6], cancelled_instances: ['xVRYunhN', '8JbzAb1O', 'NC4bO5oA', 'SmvUWBqh', 'mR12Uiqh'], interval: { quantity_of_weeks: 18, weekdays: ['Saturday', 'Friday', 'Tuesday', 'Sunday', 'Wednesday', 'Sunday', 'Friday'] }, recurrence_begins: d'2025-08-09T02:22:11.517897Z', recurrence_ends: 28 }, status: 'Scheduled', timezone: 'Asia/Anadyr', title: '1U8KHzm9' }, { all_day: true, begins: d'2025-08-09T02:22:11.518313Z', colors: { active: 'fcXpeBIn', hover: 'rePzkTJF', main: 'eymwbhSv' }, description: 'dRaeG4Wy', duration: 3h38m1s399ms517µs549ns, employees: [employee:31, employee:5], id: appointment:12, location: site:4, multi_day: false, offset_ms: 67, recurrence_rule: NULL, status: 'OnDeck', timezone: 'W-SU', title: 'OIwdviSg' }, { all_day: true, begins: d'2025-08-09T02:22:11.518457Z', colors: { active: 'cWjbDl9j', hover: 'w7FBiph0', main: 'DjPG2Yja' }, description: 'kH1DXhrA', duration: 20h38m34s794ms657µs253ns, employees: [employee:28, employee:18], id: appointment:13, location: site:40, multi_day: true, offset_ms: 71, recurrence_rule: NULL, status: 'OnDeck', timezone: 'Europe/Guernsey', title: '9OVkk1yX' } ];

-- ------------------------------
-- TABLE: company
-- ------------------------------

DEFINE TABLE company TYPE NORMAL SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Company') = true;

DEFINE FIELD account_name_format ON company TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD date_display_style ON company TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD default_account_type ON company TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD default_area_code ON company TYPE int DEFAULT 0 PERMISSIONS FULL;
DEFINE FIELD default_tax ON company TYPE record<tax_rate> DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD default_tax_location ON company TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD email ON company TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD fax ON company TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD has_attach_order_to_appointment_emails ON company TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD has_auto_commission ON company TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD has_auto_daylight_savings ON company TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD has_auto_fms_tracking ON company TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD has_notifications ON company TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD has_required_email ON company TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD has_required_lead_source ON company TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD has_sort_service_items_alphabetically ON company TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD legal_name ON company TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD lookup_formatting ON company TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD merchant_service_provider ON company TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD phones ON company TYPE array<{ can_call: bool, can_text: bool, main: bool, number: string, phone_type: string }> DEFAULT [] PERMISSIONS FULL;
DEFINE FIELD phones[*] ON company TYPE { can_call: bool, can_text: bool, main: bool, number: string, phone_type: string } PERMISSIONS FULL;
DEFINE FIELD postal_code_lookup ON company TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD reference_number ON company TYPE int DEFAULT 0 PERMISSIONS FULL;
DEFINE FIELD schedule_interval ON company TYPE int DEFAULT 0 PERMISSIONS FULL;
DEFINE FIELD tax_id ON company TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD time_zone ON company TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD website ON company TYPE string DEFAULT '' PERMISSIONS FULL;



-- ------------------------------
-- TABLE DATA: company
-- ------------------------------


-- ------------------------------
-- TABLE: employee
-- ------------------------------

DEFINE TABLE employee TYPE NORMAL SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Employee') = true;

DEFINE FIELD active ON employee TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD address ON employee TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD attendance ON employee TYPE array<datetime> DEFAULT [] PERMISSIONS FULL;
DEFINE FIELD attendance[*] ON employee TYPE datetime PERMISSIONS FULL;
DEFINE FIELD description ON employee TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD email ON employee TYPE { domain_name: string, local: string, top_level_domain: string } DEFAULT { domain_name: '', local: '', top_level_domain: '' } PERMISSIONS FULL;
DEFINE FIELD image_url ON employee TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD is_sales_rep ON employee TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD is_technician ON employee TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD linkedin_url ON employee TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD name ON employee TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD phones ON employee TYPE array<{ can_call: bool, can_text: bool, main: bool, number: string, phone_type: string }> DEFAULT [] PERMISSIONS FULL;
DEFINE FIELD phones[*] ON employee TYPE { can_call: bool, can_text: bool, main: bool, number: string, phone_type: string } PERMISSIONS FULL;
DEFINE FIELD rate_per_hour ON employee TYPE int DEFAULT 0 PERMISSIONS FULL;
DEFINE FIELD role ON employee TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD route ON employee TYPE record<route> DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD settings ON employee TYPE { appointment_notifications: null | { all_schedule_change_notifications: string, personal_schedule_change_notifications: string }, commissions: null | { sales_rep: string, technician: string }, home_page: null | 'SalesHomeDashboard' | 'SalesHomeProducts' | 'SalesHomeServices' | 'SalesHomePackages' | 'SalesHomeTaxRates' | 'SalesLeadsOverview' | 'SalesLeadsActivities' | 'SalesLeadsCampaigns' | 'SalesLeadsDripCampaigns' | 'SalesLeadsOpportunities' | 'SalesLeadsPromotions' | 'SalesAccountsOverview' | 'SalesAccountsActivities' | 'SalesAccountsBilling' | 'SalesAccountsContracts' | 'SalesOrdersOverview' | 'SalesOrdersActivities' | 'SalesOrdersPayments' | 'SalesOrdersCommissions' | 'SalesSchedulingSchedule' | 'SalesSchedulingAppointments' | 'SalesSchedulingRecurring' | 'SalesSchedulingRoutes' | 'SalesSchedulingReminders' | 'UserHome' } DEFAULT { appointment_notifications: NULL, commissions: NULL, home_page: NULL } PERMISSIONS FULL;
DEFINE FIELD title ON employee TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD username ON employee TYPE string DEFAULT '' PERMISSIONS FULL;



-- ------------------------------
-- TABLE DATA: employee
-- ------------------------------


-- ------------------------------
-- TABLE: lead
-- ------------------------------

DEFINE TABLE lead TYPE NORMAL SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Lead') = true;

DEFINE FIELD `value` ON lead TYPE float DEFAULT 0f PERMISSIONS FULL;
DEFINE FIELD accepted ON lead TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD account_type ON lead TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD close_date ON lead TYPE null | datetime DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD color ON lead TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD company_name ON lead TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD custom_fields ON lead TYPE object DEFAULT {  } PERMISSIONS FULL;
DEFINE FIELD custom_fields[*] ON lead TYPE string PERMISSIONS FULL;
DEFINE FIELD date_added ON lead TYPE null | datetime DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD description ON lead TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD due_date ON lead TYPE null | datetime DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD email ON lead TYPE { domain_name: string, local: string, top_level_domain: string } DEFAULT { domain_name: '', local: '', top_level_domain: '' } PERMISSIONS FULL;
DEFINE FIELD favorite ON lead TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD first_name ON lead TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD has_alert ON lead TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD is_tax_exempt ON lead TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD last_name ON lead TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD lead_source ON lead TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD memo ON lead TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD needs_review ON lead TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD next_step ON lead TYPE 'InitialContact' | 'Qualified' | 'Estimate' | 'Negotiation' DEFAULT 'InitialContact' PERMISSIONS FULL;
DEFINE FIELD number ON lead TYPE null | int DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD payment_terms ON lead TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD phones ON lead TYPE array<{ can_call: bool, can_text: bool, main: bool, number: string, phone_type: string }> DEFAULT [] PERMISSIONS FULL;
DEFINE FIELD phones[*] ON lead TYPE { can_call: bool, can_text: bool, main: bool, number: string, phone_type: string } PERMISSIONS FULL;
DEFINE FIELD priority ON lead TYPE 'High' | 'Medium' | 'Low' DEFAULT 'High' PERMISSIONS FULL;
DEFINE FIELD probability ON lead TYPE float DEFAULT 0f PERMISSIONS FULL;
DEFINE FIELD sector ON lead TYPE 'Residential' | 'Commercial' DEFAULT 'Residential' PERMISSIONS FULL;
DEFINE FIELD site ON lead TYPE record<site> DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD stage ON lead TYPE 'Open' | 'InitialContact' | 'Qualified' | 'Estimate' | 'Negotiation' DEFAULT 'Open' PERMISSIONS FULL;
DEFINE FIELD status ON lead TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD subtype ON lead TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD tags ON lead TYPE array<string> DEFAULT [] PERMISSIONS FULL;
DEFINE FIELD tags[*] ON lead TYPE string PERMISSIONS FULL;
DEFINE FIELD tax_rate ON lead TYPE null | record<tax_rate> DEFAULT NULL PERMISSIONS FULL;



-- ------------------------------
-- TABLE DATA: lead
-- ------------------------------


-- ------------------------------
-- TABLE: order
-- ------------------------------

DEFINE TABLE order TYPE NORMAL SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Order') = true;

DEFINE FIELD account ON order TYPE record<account> DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD action_item ON order TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD appointment ON order TYPE record<appointment> DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD balance ON order TYPE decimal DEFAULT 0.00dec PERMISSIONS FULL;
DEFINE FIELD billed_items ON order TYPE array<{ item: record<product> | record<service>, quantity: int, taxed: bool, upsale: bool }> DEFAULT [] PERMISSIONS FULL;
DEFINE FIELD billed_items[*] ON order TYPE { item: record<product> | record<service>, quantity: int, taxed: bool, upsale: bool } PERMISSIONS FULL;
DEFINE FIELD commissions ON order TYPE array<decimal> DEFAULT [] PERMISSIONS FULL;
DEFINE FIELD commissions[*] ON order TYPE decimal PERMISSIONS FULL;
DEFINE FIELD date_created ON order TYPE datetime DEFAULT d'2024-01-01T00:00:00Z' PERMISSIONS FULL;
DEFINE FIELD discount ON order TYPE decimal DEFAULT 0.00dec PERMISSIONS FULL;
DEFINE FIELD due ON order TYPE datetime DEFAULT d'2024-01-01T00:00:00Z' PERMISSIONS FULL;
DEFINE FIELD group ON order TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD is_posted ON order TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD last_techs ON order TYPE array<record<employee>> DEFAULT [] PERMISSIONS FULL;
DEFINE FIELD last_techs[*] ON order TYPE record<employee> PERMISSIONS FULL;
DEFINE FIELD lead_source ON order TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD memo ON order TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD needs_review ON order TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD number ON order TYPE int DEFAULT 0 PERMISSIONS FULL;
DEFINE FIELD opportunity ON order TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD package ON order TYPE null | array<record<package>> DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD package[*] ON order TYPE record<package> PERMISSIONS FULL;
DEFINE FIELD payments ON order TYPE array<record<payment>> DEFAULT [] PERMISSIONS FULL;
DEFINE FIELD payments[*] ON order TYPE record<payment> PERMISSIONS FULL;
DEFINE FIELD promotion ON order TYPE null | array<record<promotion>> DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD promotion[*] ON order TYPE record<promotion> PERMISSIONS FULL;
DEFINE FIELD reference ON order TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD sales_rep ON order TYPE record<employee> DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD site ON order TYPE record<site> DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD stage ON order TYPE 'Estimate' | 'Active' | 'Invoice' DEFAULT 'Estimate' PERMISSIONS FULL;
DEFINE FIELD subgroup ON order TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD tip ON order TYPE decimal DEFAULT 0.00dec PERMISSIONS FULL;
DEFINE FIELD total ON order DEFAULT 0.00dec READONLY VALUE <future> { fn::calc_order_total(billed_items.item.defaults.price, billed_items.quantity, discount) } PERMISSIONS FULL;
DEFINE FIELD upsale ON order TYPE decimal DEFAULT 0.00dec PERMISSIONS FULL;



-- ------------------------------
-- TABLE DATA: order
-- ------------------------------


-- ------------------------------
-- TABLE: ordered
-- ------------------------------

DEFINE TABLE ordered TYPE RELATION IN account OUT order SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Ordered') = true;

DEFINE FIELD date ON ordered TYPE datetime DEFAULT d'2024-01-01T00:00:00Z' PERMISSIONS FULL;
DEFINE FIELD in ON ordered TYPE record<account> PERMISSIONS FULL;
DEFINE FIELD out ON ordered TYPE record<order> PERMISSIONS FULL;



-- ------------------------------
-- TABLE DATA: ordered
-- ------------------------------


-- ------------------------------
-- TABLE: package
-- ------------------------------

DEFINE TABLE package TYPE NORMAL SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Package') = true;

DEFINE FIELD date ON package TYPE datetime DEFAULT d'2024-01-01T00:00:00Z' PERMISSIONS FULL;



-- ------------------------------
-- TABLE DATA: package
-- ------------------------------


-- ------------------------------
-- TABLE: payment
-- ------------------------------

DEFINE TABLE payment TYPE NORMAL SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Payment') = true;

DEFINE FIELD date ON payment TYPE datetime DEFAULT d'2024-01-01T00:00:00Z' PERMISSIONS FULL;



-- ------------------------------
-- TABLE DATA: payment
-- ------------------------------


-- ------------------------------
-- TABLE: product
-- ------------------------------

DEFINE TABLE product TYPE NORMAL SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Product') = true;

DEFINE FIELD active ON product TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD commission ON product TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD defaults ON product TYPE { description: string, price: float } DEFAULT { description: '', price: 0f } PERMISSIONS FULL;
DEFINE FIELD favorite ON product TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD group ON product TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD name ON product TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD quick_code ON product TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD subgroup ON product TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD unit ON product TYPE null | string DEFAULT NULL PERMISSIONS FULL;



-- ------------------------------
-- TABLE DATA: product
-- ------------------------------


-- ------------------------------
-- TABLE: promotion
-- ------------------------------

DEFINE TABLE promotion TYPE NORMAL SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Promotion') = true;

DEFINE FIELD date ON promotion TYPE datetime DEFAULT d'2024-01-01T00:00:00Z' PERMISSIONS FULL;



-- ------------------------------
-- TABLE DATA: promotion
-- ------------------------------


-- ------------------------------
-- TABLE: represents
-- ------------------------------

DEFINE TABLE represents TYPE RELATION IN employee OUT account SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Represents') = true;

DEFINE FIELD date_started ON represents TYPE datetime DEFAULT d'2024-01-01T00:00:00Z' PERMISSIONS FULL;
DEFINE FIELD in ON represents TYPE record<employee> PERMISSIONS FULL;
DEFINE FIELD out ON represents TYPE record<account> PERMISSIONS FULL;



-- ------------------------------
-- TABLE DATA: represents
-- ------------------------------


-- ------------------------------
-- TABLE: route
-- ------------------------------

DEFINE TABLE route TYPE NORMAL SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Route') = true;

DEFINE FIELD active ON route TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD color ON route TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD default_duration_hours ON route TYPE int DEFAULT 0 PERMISSIONS FULL;
DEFINE FIELD icon ON route TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD name ON route TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD phone ON route TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD position ON route TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD service_route ON route TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD tags ON route TYPE array<string> DEFAULT [] PERMISSIONS FULL;
DEFINE FIELD tags[*] ON route TYPE string PERMISSIONS FULL;
DEFINE FIELD techs ON route TYPE null | array<record<employee>> DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD techs[*] ON route TYPE record<employee> PERMISSIONS FULL;



-- ------------------------------
-- TABLE DATA: route
-- ------------------------------


-- ------------------------------
-- TABLE: service
-- ------------------------------

DEFINE TABLE service TYPE NORMAL SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Service') = true;

DEFINE FIELD active ON service TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD average_time ON service TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD commission ON service TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD defaults ON service TYPE { description: string, price: float } DEFAULT { description: '', price: 0f } PERMISSIONS FULL;
DEFINE FIELD favorite ON service TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD group ON service TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD name ON service TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD quick_code ON service TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD subgroup ON service TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD unit ON service TYPE null | string DEFAULT NULL PERMISSIONS FULL;



-- ------------------------------
-- TABLE DATA: service
-- ------------------------------


-- ------------------------------
-- TABLE: site
-- ------------------------------

DEFINE TABLE site TYPE NORMAL SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Site') = true;

DEFINE FIELD address_line1 ON site TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD address_line2 ON site TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD administrative_area_level1 ON site TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD administrative_area_level2 ON site TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD administrative_area_level3 ON site TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD coordinates ON site TYPE { lat: float, lng: float } DEFAULT { lat: 0f, lng: 0f } PERMISSIONS FULL;
DEFINE FIELD country ON site TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD locality ON site TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD postal_code ON site TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD postal_code_suffix ON site TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD sublocality_level1 ON site TYPE string DEFAULT '' PERMISSIONS FULL;



-- ------------------------------
-- TABLE DATA: site
-- ------------------------------


-- ------------------------------
-- TABLE: tax_rate
-- ------------------------------

DEFINE TABLE tax_rate TYPE NORMAL SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('TaxRate') = true;

DEFINE FIELD city ON tax_rate TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD county ON tax_rate TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD description ON tax_rate TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD is_active ON tax_rate TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD name ON tax_rate TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD state ON tax_rate TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD tax_agency ON tax_rate TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD tax_components ON tax_rate TYPE object DEFAULT {  } PERMISSIONS FULL;
DEFINE FIELD tax_components[*] ON tax_rate TYPE float PERMISSIONS FULL;
DEFINE FIELD zip ON tax_rate TYPE int DEFAULT 0 PERMISSIONS FULL;



-- ------------------------------
-- TABLE DATA: tax_rate
-- ------------------------------


-- ------------------------------
-- TABLE: user
-- ------------------------------

DEFINE TABLE user TYPE NORMAL SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('User') = true;

DEFINE FIELD email ON user TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD email_token ON user TYPE null | string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD first_name ON user TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD last_name ON user TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD metadata ON user TYPE null | { created_at: datetime, is_active: bool, last_login: null | datetime, roles: array<string> } DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD password ON user TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD permissions ON user TYPE { applications: array<'Sales' | 'Accounting' | 'Errand' | 'HumanResources' | 'Logistics' | 'Marketing' | 'Website'>, data: array<'Account' | 'Appointment' | 'Lead' | 'TaxRate' | 'Site' | 'Employee' | 'Route' | 'Company' | 'Product' | 'Service' | 'User' | 'Order' | 'Payment' | 'Package' | 'Promotion' | 'Represents' | 'Ordered'>, pages: array<'SalesHomeDashboard' | 'SalesHomeProducts' | 'SalesHomeServices' | 'SalesHomePackages' | 'SalesHomeTaxRates' | 'SalesLeadsOverview' | 'SalesLeadsActivities' | 'SalesLeadsCampaigns' | 'SalesLeadsDripCampaigns' | 'SalesLeadsOpportunities' | 'SalesLeadsPromotions' | 'SalesAccountsOverview' | 'SalesAccountsActivities' | 'SalesAccountsBilling' | 'SalesAccountsContracts' | 'SalesOrdersOverview' | 'SalesOrdersActivities' | 'SalesOrdersPayments' | 'SalesOrdersCommissions' | 'SalesSchedulingSchedule' | 'SalesSchedulingAppointments' | 'SalesSchedulingRecurring' | 'SalesSchedulingRoutes' | 'SalesSchedulingReminders' | 'UserHome'> } DEFAULT { applications: [], data: ['Account', 'Appointment', 'Lead', 'TaxRate', 'Site', 'Employee', 'Route', 'Company', 'Product', 'Service', 'User', 'Order', 'Payment', 'Package', 'Promotion', 'Represents', 'Ordered'], pages: [] } PERMISSIONS FULL;
DEFINE FIELD role ON user TYPE 'Administrator' | 'SalesRepresentative' | 'Technician' | 'HumanResources' | 'InformationTechnology' DEFAULT 'Technician' PERMISSIONS FULL;
DEFINE FIELD settings ON user TYPE { appointment_notifications: null | { all_schedule_change_notifications: string, personal_schedule_change_notifications: string }, commissions: null | { sales_rep: string, technician: string }, home_page: null | 'SalesHomeDashboard' | 'SalesHomeProducts' | 'SalesHomeServices' | 'SalesHomePackages' | 'SalesHomeTaxRates' | 'SalesLeadsOverview' | 'SalesLeadsActivities' | 'SalesLeadsCampaigns' | 'SalesLeadsDripCampaigns' | 'SalesLeadsOpportunities' | 'SalesLeadsPromotions' | 'SalesAccountsOverview' | 'SalesAccountsActivities' | 'SalesAccountsBilling' | 'SalesAccountsContracts' | 'SalesOrdersOverview' | 'SalesOrdersActivities' | 'SalesOrdersPayments' | 'SalesOrdersCommissions' | 'SalesSchedulingSchedule' | 'SalesSchedulingAppointments' | 'SalesSchedulingRecurring' | 'SalesSchedulingRoutes' | 'SalesSchedulingReminders' | 'UserHome' } DEFAULT { appointment_notifications: NULL, commissions: NULL, home_page: NULL } PERMISSIONS FULL;



-- ------------------------------
-- TABLE DATA: user
-- ------------------------------


