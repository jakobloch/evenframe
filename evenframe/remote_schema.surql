-- ------------------------------
-- OPTION
-- ------------------------------

OPTION IMPORT;

-- ------------------------------
-- ACCESSES
-- ------------------------------

DEFINE ACCESS user ON DATABASE TYPE RECORD SIGNUP (CREATE user SET email = $email, password = crypto::argon2::generate($password)) SIGNIN (SELECT * FROM user WHERE email = $email AND crypto::argon2::compare(password, $password)) WITH JWT ALGORITHM HS512 KEY 'aGf3aLGYQ5TB2zNua7aIH9NuCrdxRigatdeK95e56KVLcGbPzVVpGvgx2OzmiQRor29a9CERYqvMYShWS9Ef5z2jmpSBQL8skMOD0bE9Ie1il147zmev2augW5sB1Mf3' WITH ISSUER KEY 'aGf3aLGYQ5TB2zNua7aIH9NuCrdxRigatdeK95e56KVLcGbPzVVpGvgx2OzmiQRor29a9CERYqvMYShWS9Ef5z2jmpSBQL8skMOD0bE9Ie1il147zmev2augW5sB1Mf3' DURATION FOR TOKEN 15m, FOR SESSION 6h;

-- ------------------------------
-- TABLE: account
-- ------------------------------

DEFINE TABLE account TYPE ANY SCHEMALESS PERMISSIONS NONE;




-- ------------------------------
-- TABLE: appointment
-- ------------------------------

DEFINE TABLE appointment TYPE ANY SCHEMALESS PERMISSIONS NONE;




-- ------------------------------
-- TABLE: company
-- ------------------------------

DEFINE TABLE company TYPE ANY SCHEMALESS PERMISSIONS NONE;




-- ------------------------------
-- TABLE: employee
-- ------------------------------

DEFINE TABLE employee TYPE ANY SCHEMALESS PERMISSIONS NONE;




-- ------------------------------
-- TABLE: lead
-- ------------------------------

DEFINE TABLE lead TYPE ANY SCHEMALESS PERMISSIONS NONE;




-- ------------------------------
-- TABLE: order
-- ------------------------------

DEFINE TABLE order TYPE ANY SCHEMALESS PERMISSIONS NONE;




-- ------------------------------
-- TABLE: ordered
-- ------------------------------

DEFINE TABLE ordered TYPE ANY SCHEMALESS PERMISSIONS NONE;




-- ------------------------------
-- TABLE: package
-- ------------------------------

DEFINE TABLE package TYPE ANY SCHEMALESS PERMISSIONS NONE;




-- ------------------------------
-- TABLE: payment
-- ------------------------------

DEFINE TABLE payment TYPE ANY SCHEMALESS PERMISSIONS NONE;




-- ------------------------------
-- TABLE: product
-- ------------------------------

DEFINE TABLE product TYPE ANY SCHEMALESS PERMISSIONS NONE;




-- ------------------------------
-- TABLE: promotion
-- ------------------------------

DEFINE TABLE promotion TYPE ANY SCHEMALESS PERMISSIONS NONE;




-- ------------------------------
-- TABLE: represents
-- ------------------------------

DEFINE TABLE represents TYPE ANY SCHEMALESS PERMISSIONS NONE;




-- ------------------------------
-- TABLE: route
-- ------------------------------

DEFINE TABLE route TYPE ANY SCHEMALESS PERMISSIONS NONE;




-- ------------------------------
-- TABLE: service
-- ------------------------------

DEFINE TABLE service TYPE ANY SCHEMALESS PERMISSIONS NONE;




-- ------------------------------
-- TABLE: site
-- ------------------------------

DEFINE TABLE site TYPE ANY SCHEMALESS PERMISSIONS NONE;




-- ------------------------------
-- TABLE: tax_rate
-- ------------------------------

DEFINE TABLE tax_rate TYPE ANY SCHEMALESS PERMISSIONS NONE;




-- ------------------------------
-- TABLE: user
-- ------------------------------

DEFINE TABLE user TYPE ANY SCHEMALESS PERMISSIONS NONE;




