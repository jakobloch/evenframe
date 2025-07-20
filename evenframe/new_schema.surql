-- ------------------------------
-- OPTION
-- ------------------------------

OPTION IMPORT;

-- ------------------------------
-- TABLE: account
-- ------------------------------

DEFINE TABLE account TYPE NORMAL SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Account') = true;

DEFINE FIELD account_type ON account TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD colors ON account TYPE { active: string, hover: string, main: string } DEFAULT { active: '', hover: '', main: '' } PERMISSIONS FULL;
DEFINE FIELD company_name ON account TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD custom_fields ON account FLEXIBLE TYPE object DEFAULT {  } PERMISSIONS FULL;
DEFINE FIELD custom_fields[*] ON account TYPE string PERMISSIONS FULL;
DEFINE FIELD date_added ON account TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD email ON account TYPE { can_email: bool, email_string: string } DEFAULT { can_email: false, email_string: '' } PERMISSIONS FULL;
DEFINE FIELD first_name ON account TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD has_alert ON account TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD is_tax_exempt ON account TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD last_name ON account TYPE string DEFAULT '' PERMISSIONS FULL;
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
-- TABLE: appointment
-- ------------------------------

DEFINE TABLE appointment TYPE NORMAL SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Appointment') = true;

DEFINE FIELD all_day ON appointment TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD begins ON appointment TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD colors ON appointment TYPE { active: string, hover: string, main: string } DEFAULT { active: '', hover: '', main: '' } PERMISSIONS FULL;
DEFINE FIELD description ON appointment TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD duration ON appointment TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD employees ON appointment TYPE array<record<employee>> DEFAULT [] PERMISSIONS FULL;
DEFINE FIELD employees[*] ON appointment TYPE record<employee> PERMISSIONS FULL;
DEFINE FIELD location ON appointment TYPE record<site> DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD multi_day ON appointment TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD offset_ms ON appointment TYPE int DEFAULT 0 PERMISSIONS FULL;
DEFINE FIELD recurrence_rule ON appointment TYPE null | { additional_instances: null | array<record<appointment>>, cancelled_instances: null | array<string>, interval: { quantity_of_days: int } | { quantity_of_weeks: int, weekdays: array<'Monday' | 'Tuesday' | 'Wednesday' | 'Thursday' | 'Friday' | 'Saturday' | 'Sunday'> } | { day: int, name: string, quantity_of_months: int } | { quantity_of_years: int }, recurrence_begins: string, recurrence_ends: null | int | string } DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD status ON appointment TYPE 'Scheduled' | 'OnDeck' | 'Waiting' DEFAULT 'Scheduled' PERMISSIONS FULL;
DEFINE FIELD timezone ON appointment TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD title ON appointment TYPE string DEFAULT '' PERMISSIONS FULL;



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
-- TABLE: employee
-- ------------------------------

DEFINE TABLE employee TYPE NORMAL SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Employee') = true;

DEFINE FIELD active ON employee TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD address ON employee TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD attendance ON employee TYPE array<string> DEFAULT [] PERMISSIONS FULL;
DEFINE FIELD attendance[*] ON employee TYPE string PERMISSIONS FULL;
DEFINE FIELD description ON employee TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD email ON employee TYPE { can_email: bool, email_string: string } DEFAULT { can_email: false, email_string: '' } PERMISSIONS FULL;
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
-- TABLE: lead
-- ------------------------------

DEFINE TABLE lead TYPE NORMAL SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Lead') = true;

DEFINE FIELD `value` ON lead TYPE float DEFAULT 0 PERMISSIONS FULL;
DEFINE FIELD accepted ON lead TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD account_type ON lead TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD close_date ON lead TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD color ON lead TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD company_name ON lead TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD custom_fields ON lead TYPE object DEFAULT {  } PERMISSIONS FULL;
DEFINE FIELD custom_fields[*] ON lead TYPE string PERMISSIONS FULL;
DEFINE FIELD date_added ON lead TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD description ON lead TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD due_date ON lead TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD email ON lead TYPE { can_email: bool, email_string: string } DEFAULT { can_email: false, email_string: '' } PERMISSIONS FULL;
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
DEFINE FIELD probability ON lead TYPE float DEFAULT 0 PERMISSIONS FULL;
DEFINE FIELD sector ON lead TYPE 'Residential' | 'Commercial' DEFAULT 'Residential' PERMISSIONS FULL;
DEFINE FIELD site ON lead TYPE record<site> DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD stage ON lead TYPE 'Open' | 'InitialContact' | 'Qualified' | 'Estimate' | 'Negotiation' DEFAULT 'Open' PERMISSIONS FULL;
DEFINE FIELD status ON lead TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD subtype ON lead TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD tags ON lead TYPE array<string> DEFAULT [] PERMISSIONS FULL;
DEFINE FIELD tags[*] ON lead TYPE string PERMISSIONS FULL;
DEFINE FIELD tax_rate ON lead TYPE null | record<tax_rate> DEFAULT NULL PERMISSIONS FULL;



-- ------------------------------
-- TABLE: order
-- ------------------------------

DEFINE TABLE order TYPE NORMAL SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Order') = true;

DEFINE FIELD account ON order TYPE record<account> DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD action_item ON order TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD appointment ON order TYPE record<appointment> DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD balance ON order TYPE float DEFAULT 0 PERMISSIONS FULL;
DEFINE FIELD billed_items ON order TYPE array<{ item: record<product> | record<service>, quantity: int, taxed: bool, upsale: bool }> DEFAULT [] PERMISSIONS FULL;
DEFINE FIELD billed_items[*] ON order TYPE { item: record<product> | record<service>, quantity: int, taxed: bool, upsale: bool } PERMISSIONS FULL;
DEFINE FIELD commissions ON order TYPE array<float> DEFAULT [] PERMISSIONS FULL;
DEFINE FIELD commissions[*] ON order TYPE float PERMISSIONS FULL;
DEFINE FIELD date_created ON order TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD discount ON order TYPE float DEFAULT 0 PERMISSIONS FULL;
DEFINE FIELD due ON order TYPE string DEFAULT '' PERMISSIONS FULL;
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
DEFINE FIELD tip ON order TYPE float DEFAULT 0 PERMISSIONS FULL;
DEFINE FIELD total ON order DEFAULT 0 READONLY VALUE <future> { fn::calc_order_total(billed_items.item.defaults.price, billed_items.quantity, discount) } PERMISSIONS FULL;
DEFINE FIELD upsale ON order TYPE float DEFAULT 0 PERMISSIONS FULL;



-- ------------------------------
-- TABLE: ordered
-- ------------------------------

DEFINE TABLE ordered TYPE RELATION IN account OUT order SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Ordered') = true;

DEFINE FIELD date ON ordered TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD in ON ordered TYPE record<account> PERMISSIONS FULL;
DEFINE FIELD out ON ordered TYPE record<order> PERMISSIONS FULL;



-- ------------------------------
-- TABLE: package
-- ------------------------------

DEFINE TABLE package TYPE NORMAL SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Package') = true;

DEFINE FIELD date ON package TYPE string DEFAULT '' PERMISSIONS FULL;



-- ------------------------------
-- TABLE: payment
-- ------------------------------

DEFINE TABLE payment TYPE NORMAL SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Payment') = true;

DEFINE FIELD date ON payment TYPE string DEFAULT '' PERMISSIONS FULL;



-- ------------------------------
-- TABLE: product
-- ------------------------------

DEFINE TABLE product TYPE NORMAL SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Product') = true;

DEFINE FIELD active ON product TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD commission ON product TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD defaults ON product TYPE { description: string, price: float } DEFAULT { description: '', price: 0 } PERMISSIONS FULL;
DEFINE FIELD favorite ON product TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD group ON product TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD name ON product TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD quick_code ON product TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD subgroup ON product TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD unit ON product TYPE null | string DEFAULT NULL PERMISSIONS FULL;



-- ------------------------------
-- TABLE: promotion
-- ------------------------------

DEFINE TABLE promotion TYPE NORMAL SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Promotion') = true;

DEFINE FIELD date ON promotion TYPE string DEFAULT '' PERMISSIONS FULL;



-- ------------------------------
-- TABLE: represents
-- ------------------------------

DEFINE TABLE represents TYPE RELATION IN employee OUT account SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Represents') = true;

DEFINE FIELD date_started ON represents TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD in ON represents TYPE record<employee> PERMISSIONS FULL;
DEFINE FIELD out ON represents TYPE record<account> PERMISSIONS FULL;



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
-- TABLE: service
-- ------------------------------

DEFINE TABLE service TYPE NORMAL SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Service') = true;

DEFINE FIELD active ON service TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD average_time ON service TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD commission ON service TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD defaults ON service TYPE { description: string, price: float } DEFAULT { description: '', price: 0 } PERMISSIONS FULL;
DEFINE FIELD favorite ON service TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD group ON service TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD name ON service TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD quick_code ON service TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD subgroup ON service TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD unit ON service TYPE null | string DEFAULT NULL PERMISSIONS FULL;



-- ------------------------------
-- TABLE: site
-- ------------------------------

DEFINE TABLE site TYPE NORMAL SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('Site') = true;

DEFINE FIELD address_line1 ON site TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD address_line2 ON site TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD administrative_area_level1 ON site TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD administrative_area_level2 ON site TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD administrative_area_level3 ON site TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD coordinates ON site TYPE { lat: float, lng: float } DEFAULT { lat: 0, lng: 0 } PERMISSIONS FULL;
DEFINE FIELD country ON site TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD locality ON site TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD postal_code ON site TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD postal_code_suffix ON site TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD sublocality_level1 ON site TYPE string DEFAULT '' PERMISSIONS FULL;



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
-- TABLE: user
-- ------------------------------

DEFINE TABLE user TYPE NORMAL SCHEMAFULL CHANGEFEED 3d PERMISSIONS FOR select, create, update, delete WHERE $auth.permissions.data.any('User') = true;

DEFINE FIELD email ON user TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD email_token ON user TYPE null | string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD first_name ON user TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD last_name ON user TYPE string DEFAULT '' PERMISSIONS FULL;
DEFINE FIELD metadata ON user TYPE null | { created_at: string, is_active: bool, last_login: null | string, roles: array<string> } DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD password ON user TYPE null | string DEFAULT NULL PERMISSIONS FULL;
DEFINE FIELD permissions ON user TYPE { applications: array<'Sales' | 'Accounting' | 'Errand' | 'HumanResources' | 'Logistics' | 'Marketing' | 'Website'>, data: array<'Account' | 'Appointment' | 'Lead' | 'TaxRate' | 'Site' | 'Employee' | 'Route' | 'Company' | 'Product' | 'Service' | 'User' | 'Order' | 'Payment' | 'Package' | 'Promotion' | 'Represents' | 'Ordered'>, pages: array<'SalesHomeDashboard' | 'SalesHomeProducts' | 'SalesHomeServices' | 'SalesHomePackages' | 'SalesHomeTaxRates' | 'SalesLeadsOverview' | 'SalesLeadsActivities' | 'SalesLeadsCampaigns' | 'SalesLeadsDripCampaigns' | 'SalesLeadsOpportunities' | 'SalesLeadsPromotions' | 'SalesAccountsOverview' | 'SalesAccountsActivities' | 'SalesAccountsBilling' | 'SalesAccountsContracts' | 'SalesOrdersOverview' | 'SalesOrdersActivities' | 'SalesOrdersPayments' | 'SalesOrdersCommissions' | 'SalesSchedulingSchedule' | 'SalesSchedulingAppointments' | 'SalesSchedulingRecurring' | 'SalesSchedulingRoutes' | 'SalesSchedulingReminders' | 'UserHome'> } DEFAULT { applications: [], data: ['Account', 'Appointment', 'Lead', 'TaxRate', 'Site', 'Employee', 'Route', 'Company', 'Product', 'Service', 'User', 'Order', 'Payment', 'Package', 'Promotion', 'Represents', 'Ordered'], pages: [] } PERMISSIONS FULL;
DEFINE FIELD role ON user TYPE 'Administrator' | 'SalesRepresentative' | 'Technician' | 'HumanResources' | 'InformationTechnology' DEFAULT 'Technician' PERMISSIONS FULL;
DEFINE FIELD settings ON user TYPE { appointment_notifications: null | { all_schedule_change_notifications: string, personal_schedule_change_notifications: string }, commissions: null | { sales_rep: string, technician: string }, home_page: null | 'SalesHomeDashboard' | 'SalesHomeProducts' | 'SalesHomeServices' | 'SalesHomePackages' | 'SalesHomeTaxRates' | 'SalesLeadsOverview' | 'SalesLeadsActivities' | 'SalesLeadsCampaigns' | 'SalesLeadsDripCampaigns' | 'SalesLeadsOpportunities' | 'SalesLeadsPromotions' | 'SalesAccountsOverview' | 'SalesAccountsActivities' | 'SalesAccountsBilling' | 'SalesAccountsContracts' | 'SalesOrdersOverview' | 'SalesOrdersActivities' | 'SalesOrdersPayments' | 'SalesOrdersCommissions' | 'SalesSchedulingSchedule' | 'SalesSchedulingAppointments' | 'SalesSchedulingRecurring' | 'SalesSchedulingRoutes' | 'SalesSchedulingReminders' | 'UserHome' } DEFAULT { appointment_notifications: NULL, commissions: NULL, home_page: NULL } PERMISSIONS FULL;



