-- ------------------------------
-- OPTION
-- ------------------------------

OPTION IMPORT;

-- ------------------------------
-- ACCESSES
-- ------------------------------

DEFINE ACCESS user ON DATABASE TYPE RECORD SIGNUP (CREATE user SET email = $email, password = crypto::argon2::generate($password)) SIGNIN (SELECT * FROM user WHERE email = $email AND crypto::argon2::compare(password, $password)) WITH JWT ALGORITHM HS512 KEY 'E0IEedJkaSWmjHkLWhW68JYNuxXpoR2mn75VQBNB1oxQUndXc1oC2repjYfYyRaPfLeL3yA6TkP7xrJo0BETRfZHpuGB2VRWDzfPbZ5PL7klvKtsrI8hJwn6JJcMEJUV' WITH ISSUER KEY 'E0IEedJkaSWmjHkLWhW68JYNuxXpoR2mn75VQBNB1oxQUndXc1oC2repjYfYyRaPfLeL3yA6TkP7xrJo0BETRfZHpuGB2VRWDzfPbZ5PL7klvKtsrI8hJwn6JJcMEJUV' DURATION FOR TOKEN 15m, FOR SESSION 6h;

